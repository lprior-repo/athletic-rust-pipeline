use super::{ledger, TeamsSource};
use crate::restate_services::wire::{
    OriginHeldContinuationRequest, TeamsContinuationOutcome, TeamsSourceRequest,
};
use crate::restate_services::{blocking, job_error, JobError};
use census_crawl::net::Fetcher;
use restate_sdk::prelude::*;
use std::sync::Arc;

impl TeamsSource {
    #[tracing::instrument(skip_all)]
    pub(super) async fn authorize_origin_continuation(
        &self,
        ctx: &ObjectContext<'_>,
        request: &OriginHeldContinuationRequest,
    ) -> Result<ledger::origin_continuation::Authorization, HandlerError> {
        let operation = ctx.key().to_owned();
        let request = request.clone();
        let Json(authorization) = ctx
            .run(move || async move {
                let admission = self.admit(&operation).await?;
                let fetcher = self
                    .owner
                    .fetcher(
                        &request.original.jurisdiction.authorized_hosts,
                        request.original.jurisdiction.source_parallelism,
                    )
                    .await?;
                let store = Arc::clone(&self.owner.store);
                let origin = source_origin(&request.original);
                blocking(Arc::clone(self.jobs.region()), move || {
                    let _admission = admission;
                    let authorization = ledger::origin_continuation::validate(
                        &store.snapshot(),
                        &operation,
                        Some(&request.original),
                    )?;
                    if authorization.original_digest != request.original_outcome_digest {
                        return Err(JobError::Terminal {
                            message: "origin continuation original outcome digest mismatch".into(),
                        });
                    }
                    if ledger::origin_continuation::status(&store, &operation)?.is_none() {
                        fetcher.claim_origin(&origin).map_err(ownership_failure)?;
                    }
                    ledger::origin_continuation::authorize(
                        &store,
                        &operation,
                        &request.original,
                        &request.original_outcome_digest,
                    )
                })
                .await
                .map(Json)
                .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(authorization)
    }

    #[tracing::instrument(skip_all)]
    pub(super) async fn claim_continuation_origin(
        &self,
        fetcher: &Arc<Fetcher>,
        request: &TeamsSourceRequest,
    ) -> Result<(), HandlerError> {
        let fetcher = Arc::clone(fetcher);
        let origin = source_origin(request);
        blocking(Arc::clone(self.jobs.region()), move || {
            fetcher.claim_origin(&origin).map_err(ownership_failure)
        })
        .await
        .map_err(job_error)
    }

    #[tracing::instrument(skip_all)]
    pub(super) async fn current_source_outcome(
        &self,
        operation: String,
    ) -> Result<Option<TeamsContinuationOutcome>, JobError> {
        let permit = self
            .jobs
            .permit_control()
            .await
            .map_err(|error| JobError::Terminal {
                message: error.to_string(),
            })?;
        let store = Arc::clone(&self.owner.store);
        blocking(
            Arc::clone(self.jobs.region()),
            move || -> Result<Option<TeamsContinuationOutcome>, JobError> {
                let _permit = permit;
                match ledger::origin_continuation::status(&store, &operation)? {
                    Some(outcome) => {
                        let authorization = ledger::origin_continuation::validate(
                            &store.snapshot(),
                            &operation,
                            None,
                        )?;
                        Ok(Some(
                            crate::restate_services::wire::TeamsContinuationOutcome {
                                original_outcome_digest: authorization.original_digest,
                                outcome,
                            },
                        ))
                    }
                    None => Ok(None),
                }
            },
        )
        .await
    }
}

fn ownership_failure(error: census_crawl::net::FetchError) -> JobError {
    JobError::Terminal {
        message: error.to_string(),
    }
}

fn source_origin(request: &TeamsSourceRequest) -> String {
    format!(
        "https://{}",
        census_crawl::milesplit::Site::for_jurisdiction(request.jurisdiction.jurisdiction,).host()
    )
}
