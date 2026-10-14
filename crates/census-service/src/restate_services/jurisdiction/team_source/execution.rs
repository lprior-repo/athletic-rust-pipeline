use super::{ledger, TeamsSource};
use crate::restate_services::job_error;
use crate::restate_services::wire::{TeamsSourceOutcome, TeamsSourceRequest};
use restate_sdk::prelude::*;
use std::sync::Arc;

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
        let admission = Arc::new(self.admit(ctx.key()).await?);
        let registered = Arc::new(self.register(&ctx, &request, &admission).await?);
        let reserved = self
            .begin_attempt(
                ctx.key().to_string(),
                Arc::clone(&request),
                Arc::clone(&registered),
                Arc::clone(&admission),
            )
            .await
            .map_err(job_error)?;
        self.execute_reserved(&ctx, &request, admission, &registered, reserved)
            .await
    }

    async fn journaled_status(
        &self,
        ctx: &ObjectContext<'_>,
        request: &Arc<TeamsSourceRequest>,
    ) -> Result<Option<TeamsSourceOutcome>, HandlerError> {
        let operation = ctx.key().to_string();
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

    async fn execute_reserved(
        &self,
        ctx: &ObjectContext<'_>,
        request: &TeamsSourceRequest,
        _admission: Arc<super::admission::WorkAdmission>,
        _registered: &ledger::Identity,
        reserved: ledger::Admission,
    ) -> Result<Json<TeamsSourceOutcome>, HandlerError> {
        let (attempt, observed_on) = match reserved {
            ledger::Admission::Settled(outcome) => return Ok(Json(outcome)),
            ledger::Admission::Reserved {
                attempt,
                observed_on,
            } => (attempt, observed_on),
        };
        #[cfg(feature = "native-fault-injection")]
        super::native_boundary::wait(
            &self.jobs,
            Arc::clone(&_admission),
            ctx.key(),
            attempt,
            _registered,
        )
        .await?;
        let (result, conditions) = self
            .acquire(request, observed_on)
            .await
            .map_err(job_error)?;
        match self
            .finish_attempt(
                ctx.key().to_string(),
                attempt,
                result,
                _admission,
                conditions,
            )
            .await
            .map_err(job_error)?
        {
            ledger::Completion::Settled(outcome) => Ok(Json(outcome)),
            ledger::Completion::Retry(error) => Err(job_error(error)),
        }
    }
}
