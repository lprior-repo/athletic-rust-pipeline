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
        if let Some(outcome) =
            ledger::status(&self.owner.store, ctx.key(), Some(&request)).map_err(job_error)?
        {
            return Ok(Json(outcome));
        }
        let request = Arc::new(request);
        let admission = Arc::new(self.admit(ctx.key()).await?);
        let registered = self.register(&ctx, &request, &admission).await?;
        let reserved = ledger::begin(&self.owner.store, ctx.key(), &request, &registered)
            .map_err(job_error)?;
        self.execute_reserved(&ctx, &request, admission, &registered, reserved)
            .await
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
        let result = self.acquire(request, observed_on).await;
        match ledger::finish(&self.owner.store, ctx.key(), attempt, result).map_err(job_error)? {
            ledger::Completion::Settled(outcome) => Ok(Json(outcome)),
            ledger::Completion::Retry(error) => Err(job_error(error)),
        }
    }
}
