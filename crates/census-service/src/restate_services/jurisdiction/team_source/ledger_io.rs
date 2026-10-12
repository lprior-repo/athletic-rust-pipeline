use std::sync::Arc;

use super::{admission::WorkAdmission, ledger, TeamsSource};
use crate::restate_services::wire::{
    TeamsSourceInspection, TeamsSourceOutcome, TeamsSourceRequest,
};
use crate::restate_services::{blocking, JobError};

impl TeamsSource {
    #[tracing::instrument(skip_all)]
    pub(super) async fn ledger_status(
        &self,
        operation: String,
        request: Option<Arc<TeamsSourceRequest>>,
    ) -> Result<Option<TeamsSourceOutcome>, JobError> {
        let store = Arc::clone(&self.owner.store);
        let region = Arc::clone(self.jobs.region());
        let permit = self
            .jobs
            .permit()
            .await
            .map_err(|error| JobError::Terminal {
                message: error.to_string(),
            })?;
        blocking(region, move || {
            let _permit = permit;
            ledger::status(&store, &operation, request.as_deref())
        })
        .await
    }

    #[tracing::instrument(skip_all)]
    pub(super) async fn ledger_inspection(
        &self,
        operation: String,
    ) -> Result<TeamsSourceInspection, JobError> {
        let store = Arc::clone(&self.owner.store);
        let region = Arc::clone(self.jobs.region());
        let permit = self
            .jobs
            .permit()
            .await
            .map_err(|error| JobError::Terminal {
                message: error.to_string(),
            })?;
        blocking(region, move || {
            let _permit = permit;
            ledger::inspection(&store, &operation, None)
        })
        .await
    }

    #[tracing::instrument(skip_all)]
    pub(super) async fn begin_attempt(
        &self,
        operation: String,
        request: Arc<TeamsSourceRequest>,
        registered: Arc<ledger::Identity>,
        admission: Arc<WorkAdmission>,
    ) -> Result<ledger::Admission, JobError> {
        let store = Arc::clone(&self.owner.store);
        blocking(Arc::clone(self.jobs.region()), move || {
            let _admission = admission;
            ledger::begin(&store, &operation, &request, &registered)
        })
        .await
    }

    #[tracing::instrument(skip_all)]
    pub(super) async fn finish_attempt(
        &self,
        operation: String,
        attempt: u8,
        result: ledger::Attempt,
        admission: Arc<WorkAdmission>,
    ) -> Result<ledger::Completion, JobError> {
        let store = Arc::clone(&self.owner.store);
        blocking(Arc::clone(self.jobs.region()), move || {
            let _admission = admission;
            ledger::finish(&store, &operation, attempt, result)
        })
        .await
    }
}

#[cfg(test)]
mod tests;
