use super::execution_ledger::ExecutionLedger;
use super::{ledger, TeamsSource};
use crate::restate_services::wire::{TeamsSourceOutcome, TeamsSourceRequest};
use crate::restate_services::{job_error, JobError};
use restate_sdk::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
enum AttemptResult {
    Settled(TeamsSourceOutcome),
    Retry { message: String },
}

struct AdmittedAttempt {
    operation: String,
    attempt: u8,
    observed_on: String,
    admission: Arc<super::admission::WorkAdmission>,
    scope: ExecutionLedger,
}

impl TeamsSource {
    pub(super) async fn execute(
        &self,
        ctx: ObjectContext<'_>,
        request: TeamsSourceRequest,
    ) -> Result<Json<TeamsSourceOutcome>, HandlerError> {
        self.execute_in(ctx, request, ExecutionLedger::Original)
            .await
    }

    pub(super) async fn execute_in(
        &self,
        ctx: ObjectContext<'_>,
        request: TeamsSourceRequest,
        scope: ExecutionLedger,
    ) -> Result<Json<TeamsSourceOutcome>, HandlerError> {
        crate::restate_services::limits::validate_source_parallelism(
            request.jurisdiction.source_parallelism,
        )?;
        let request = Arc::new(request);
        if let Some(outcome) = self.journaled_status(&ctx, &request, &scope).await? {
            return Ok(Json(outcome));
        }
        if let Some(outcome) = self.wait_recorded_access(&ctx, &request, &scope).await? {
            return Ok(Json(outcome));
        }
        let registered = Arc::new(self.register(&ctx, &request).await?);
        for round in 1..=ledger::MAX_ATTEMPTS {
            if round > 1 {
                if let Some(outcome) = self.wait_recorded_access(&ctx, &request, &scope).await? {
                    return Ok(Json(outcome));
                }
            }
            let baseline = self.journaled_baseline(&ctx, &scope).await?;
            let operation = ctx.key().to_owned();
            let request = Arc::clone(&request);
            let registered = Arc::clone(&registered);
            let scope = scope.clone();
            let Json(result) = ctx
                .run(move || async move {
                    self.execute_attempt(operation, request, registered, baseline, scope)
                        .await
                        .map(Json)
                })
                .retry_policy(RunRetryPolicy::new().max_attempts(1))
                .await?;
            match result {
                AttemptResult::Settled(outcome) => return Ok(Json(outcome)),
                AttemptResult::Retry { message } if round == ledger::MAX_ATTEMPTS => {
                    return Err(job_error(JobError::Transient { message }));
                }
                AttemptResult::Retry { .. } => {}
            }
        }
        Err(job_error(JobError::Terminal {
            message: "bounded source execution ended without an outcome".into(),
        }))
    }

    async fn execute_attempt(
        &self,
        operation: String,
        request: Arc<TeamsSourceRequest>,
        registered: Arc<ledger::Identity>,
        baseline: u8,
        scope: ExecutionLedger,
    ) -> Result<AttemptResult, HandlerError> {
        let admission = Arc::new(self.admit(&operation).await?);
        let fetcher = self.attempt_fetcher(&request, &scope).await?;
        let reserved = self
            .begin_at_attempt_in(
                operation.clone(),
                Arc::clone(&request),
                Arc::clone(&registered),
                Arc::clone(&admission),
                baseline,
                scope.clone(),
            )
            .await
            .map_err(job_error)?;
        let (attempt, observed_on) = match reserved {
            ledger::AdmissionReplay::Settled(outcome) => {
                return Ok(AttemptResult::Settled(outcome));
            }
            ledger::AdmissionReplay::ReplayRetry { message } => {
                return Ok(AttemptResult::Retry { message });
            }
            ledger::AdmissionReplay::Reserved {
                attempt,
                observed_on,
            } => (attempt, observed_on),
        };
        #[cfg(feature = "native-fault-injection")]
        super::native_boundary::wait(
            &self.jobs,
            Arc::clone(&admission),
            &operation,
            attempt,
            &registered,
        )
        .await?;
        self.acquire_and_finish(
            AdmittedAttempt {
                operation,
                attempt,
                observed_on,
                admission,
                scope,
            },
            request,
            &fetcher,
        )
        .await
    }

    async fn attempt_fetcher(
        &self,
        request: &TeamsSourceRequest,
        scope: &ExecutionLedger,
    ) -> Result<Arc<census_crawl::net::Fetcher>, HandlerError> {
        let fetcher = self
            .owner
            .fetcher(
                &request.jurisdiction.authorized_hosts,
                request.jurisdiction.source_parallelism,
            )
            .await?;
        if matches!(scope, ExecutionLedger::OriginContinuation(_)) {
            self.claim_continuation_origin(&fetcher, request).await?;
        }
        Ok(fetcher)
    }

    async fn acquire_and_finish(
        &self,
        admitted: AdmittedAttempt,
        request: Arc<TeamsSourceRequest>,
        fetcher: &Arc<census_crawl::net::Fetcher>,
    ) -> Result<AttemptResult, HandlerError> {
        let AdmittedAttempt {
            operation,
            attempt,
            observed_on,
            admission,
            scope,
        } = admitted;
        let (result, conditions) = self.acquire(&request, observed_on, fetcher).await;
        match self
            .finish_attempt_in(operation, attempt, result, admission, conditions, scope)
            .await
            .map_err(job_error)?
        {
            ledger::Completion::Settled(outcome) => Ok(AttemptResult::Settled(outcome)),
            ledger::Completion::Retry(JobError::Transient { message }) => {
                Ok(AttemptResult::Retry { message })
            }
            ledger::Completion::Retry(error) => Err(job_error(error)),
        }
    }

    async fn journaled_baseline(
        &self,
        ctx: &ObjectContext<'_>,
        scope: &ExecutionLedger,
    ) -> Result<u8, HandlerError> {
        let operation = ctx.key().to_owned();
        let scope = scope.clone();
        let Json(baseline) = ctx
            .run(move || async move {
                self.ledger_baseline_in(operation, scope)
                    .await
                    .map(Json)
                    .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(baseline)
    }

    async fn journaled_status(
        &self,
        ctx: &ObjectContext<'_>,
        request: &Arc<TeamsSourceRequest>,
        scope: &ExecutionLedger,
    ) -> Result<Option<TeamsSourceOutcome>, HandlerError> {
        let operation = ctx.key().to_owned();
        let request = Arc::clone(request);
        let scope = scope.clone();
        let Json(outcome) = ctx
            .run(move || async move {
                self.ledger_status_in(operation, Some(request), scope)
                    .await
                    .map(Json)
                    .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(outcome)
    }
}

#[cfg(test)]
mod setup_tests;
