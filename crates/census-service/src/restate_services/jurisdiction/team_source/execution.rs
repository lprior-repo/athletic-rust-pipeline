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

impl TeamsSource {
    pub(super) async fn execute(
        &self,
        ctx: ObjectContext<'_>,
        request: TeamsSourceRequest,
    ) -> Result<Json<TeamsSourceOutcome>, HandlerError> {
        crate::restate_services::limits::validate_source_parallelism(
            request.jurisdiction.source_parallelism,
        )?;
        let request = Arc::new(request);
        if let Some(outcome) = self.journaled_status(&ctx, &request).await? {
            return Ok(Json(outcome));
        }
        if let Some(outcome) = self.wait_recorded_access(&ctx, &request).await? {
            return Ok(Json(outcome));
        }
        let registered = Arc::new(self.register(&ctx, &request).await?);
        for round in 1..=ledger::MAX_ATTEMPTS {
            if round > 1 {
                if let Some(outcome) = self.wait_recorded_access(&ctx, &request).await? {
                    return Ok(Json(outcome));
                }
            }
            let operation = ctx.key().to_owned();
            let request = Arc::clone(&request);
            let registered = Arc::clone(&registered);
            let Json(result) = ctx
                .run(move || async move {
                    self.execute_attempt(operation, request, registered)
                        .await
                        .map(Json)
                })
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
    ) -> Result<AttemptResult, HandlerError> {
        let admission = Arc::new(self.admit(&operation).await?);
        let reserved = self
            .begin_attempt(
                operation.clone(),
                Arc::clone(&request),
                Arc::clone(&registered),
                Arc::clone(&admission),
            )
            .await
            .map_err(job_error)?;
        let (attempt, observed_on) = match reserved {
            ledger::Admission::Settled(outcome) => return Ok(AttemptResult::Settled(outcome)),
            ledger::Admission::Reserved {
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
        let (result, conditions) = self
            .acquire(&request, observed_on)
            .await
            .map_err(job_error)?;
        match self
            .finish_attempt(operation, attempt, result, admission, conditions)
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

    async fn journaled_status(
        &self,
        ctx: &ObjectContext<'_>,
        request: &Arc<TeamsSourceRequest>,
    ) -> Result<Option<TeamsSourceOutcome>, HandlerError> {
        let operation = ctx.key().to_owned();
        let request = Arc::clone(request);
        let Json(outcome) = ctx
            .run(move || async move {
                self.ledger_status(operation, Some(request))
                    .await
                    .map(Json)
                    .map_err(job_error)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(outcome)
    }
}
