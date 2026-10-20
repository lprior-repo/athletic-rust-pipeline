use std::sync::Arc;

use super::{admission::WorkAdmission, ledger, TeamsSource};
use crate::restate_services::wire::{
    TeamsSourceInspection, TeamsSourceOutcome, TeamsSourceRequest,
};
use crate::restate_services::{blocking, JobError};
use crate::spawn::Reservation;

impl TeamsSource {
    #[tracing::instrument(skip_all)]
    pub(super) async fn ledger_status(
        &self,
        operation: String,
        request: Option<Arc<TeamsSourceRequest>>,
    ) -> Result<Option<TeamsSourceOutcome>, JobError> {
        let permit = self
            .jobs
            .permit()
            .await
            .map_err(|error| JobError::Terminal {
                message: error.to_string(),
            })?;
        self.read_status(permit, operation, request).await
    }

    #[tracing::instrument(skip_all)]
    pub(super) async fn ledger_status_observed(
        &self,
        operation: String,
        request: Option<Arc<TeamsSourceRequest>>,
    ) -> Result<Option<TeamsSourceOutcome>, JobError> {
        let permit = self
            .jobs
            .permit_control()
            .await
            .map_err(|error| JobError::Terminal {
                message: error.to_string(),
            })?;
        self.read_status(permit, operation, request).await
    }

    async fn read_status(
        &self,
        permit: Reservation,
        operation: String,
        request: Option<Arc<TeamsSourceRequest>>,
    ) -> Result<Option<TeamsSourceOutcome>, JobError> {
        let store = Arc::clone(&self.owner.store);
        let region = Arc::clone(self.jobs.region());
        blocking(region, move || {
            let _permit = permit;
            ledger::status(&store, &operation, request.as_deref())
        })
        .await
    }

    #[tracing::instrument(skip_all)]
    pub(super) async fn ledger_inspection_observed(
        &self,
        operation: String,
    ) -> Result<TeamsSourceInspection, JobError> {
        let permit = self
            .jobs
            .permit_control()
            .await
            .map_err(|error| JobError::Terminal {
                message: error.to_string(),
            })?;
        self.read_inspection(permit, operation).await
    }

    async fn read_inspection(
        &self,
        permit: Reservation,
        operation: String,
    ) -> Result<TeamsSourceInspection, JobError> {
        let store = Arc::clone(&self.owner.store);
        let region = Arc::clone(self.jobs.region());
        blocking(region, move || {
            let _permit = permit;
            ledger::inspection(&store, &operation, None)
        })
        .await
    }

    #[tracing::instrument(skip_all)]
    pub(super) async fn ledger_baseline(&self, operation: String) -> Result<u8, JobError> {
        let permit = self
            .jobs
            .permit_control()
            .await
            .map_err(|error| JobError::Terminal {
                message: error.to_string(),
            })?;
        let store = Arc::clone(&self.owner.store);
        blocking(Arc::clone(self.jobs.region()), move || {
            let _permit = permit;
            ledger::baseline(&store, &operation)
        })
        .await
    }

    #[tracing::instrument(skip_all)]
    pub(super) async fn begin_at_attempt(
        &self,
        operation: String,
        request: Arc<TeamsSourceRequest>,
        registered: Arc<ledger::Identity>,
        admission: Arc<WorkAdmission>,
        baseline: u8,
    ) -> Result<ledger::AdmissionReplay, JobError> {
        let store = Arc::clone(&self.owner.store);
        blocking(Arc::clone(self.jobs.region()), move || {
            let _admission = admission;
            ledger::begin_at(&store, &operation, &request, &registered, baseline)
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
        conditions: Vec<census_domain::model::SourceAccessCondition>,
    ) -> Result<ledger::Completion, JobError> {
        let store = Arc::clone(&self.owner.store);
        blocking(Arc::clone(self.jobs.region()), move || {
            let _admission = admission;
            ledger::finish(&store, &operation, attempt, result, &conditions)
        })
        .await
    }
}

#[cfg(test)]
mod tests;
