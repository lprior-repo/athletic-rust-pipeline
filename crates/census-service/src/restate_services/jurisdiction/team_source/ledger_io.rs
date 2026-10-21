use std::sync::Arc;

use super::execution_ledger::ExecutionLedger;
use super::{admission::WorkAdmission, ledger, TeamsSource};
use crate::restate_services::wire::{
    TeamsSourceInspection, TeamsSourceOutcome, TeamsSourceRequest,
};
use crate::restate_services::{blocking, JobError};
use crate::spawn::Reservation;

impl TeamsSource {
    #[cfg(test)]
    pub(super) async fn ledger_baseline(&self, operation: String) -> Result<u8, JobError> {
        self.ledger_baseline_in(operation, ExecutionLedger::Original)
            .await
    }

    #[cfg(test)]
    pub(super) async fn ledger_status(
        &self,
        operation: String,
        request: Option<Arc<TeamsSourceRequest>>,
    ) -> Result<Option<TeamsSourceOutcome>, JobError> {
        self.ledger_status_in(operation, request, ExecutionLedger::Original)
            .await
    }

    #[tracing::instrument(skip_all)]
    pub(super) async fn ledger_status_in(
        &self,
        operation: String,
        request: Option<Arc<TeamsSourceRequest>>,
        scope: ExecutionLedger,
    ) -> Result<Option<TeamsSourceOutcome>, JobError> {
        let permit = self
            .jobs
            .permit()
            .await
            .map_err(|error| JobError::Terminal {
                message: error.to_string(),
            })?;
        self.read_status(permit, operation, request, scope).await
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
        self.read_status(permit, operation, request, ExecutionLedger::Original)
            .await
    }

    async fn read_status(
        &self,
        permit: Reservation,
        operation: String,
        request: Option<Arc<TeamsSourceRequest>>,
        scope: ExecutionLedger,
    ) -> Result<Option<TeamsSourceOutcome>, JobError> {
        let store = Arc::clone(&self.owner.store);
        let region = Arc::clone(self.jobs.region());
        blocking(region, move || {
            let _permit = permit;
            scope.status(&store, &operation, request.as_deref())
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
    pub(super) async fn ledger_baseline_in(
        &self,
        operation: String,
        scope: ExecutionLedger,
    ) -> Result<u8, JobError> {
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
            scope.baseline(&store, &operation)
        })
        .await
    }

    #[tracing::instrument(skip_all)]
    #[cfg(test)]
    pub(super) async fn begin_at_attempt(
        &self,
        operation: String,
        request: Arc<TeamsSourceRequest>,
        registered: Arc<ledger::Identity>,
        admission: Arc<WorkAdmission>,
        baseline: u8,
    ) -> Result<ledger::AdmissionReplay, JobError> {
        self.begin_at_attempt_in(
            operation,
            request,
            registered,
            admission,
            baseline,
            ExecutionLedger::Original,
        )
        .await
    }

    #[tracing::instrument(skip_all)]
    pub(super) async fn begin_at_attempt_in(
        &self,
        operation: String,
        request: Arc<TeamsSourceRequest>,
        registered: Arc<ledger::Identity>,
        admission: Arc<WorkAdmission>,
        baseline: u8,
        scope: ExecutionLedger,
    ) -> Result<ledger::AdmissionReplay, JobError> {
        let store = Arc::clone(&self.owner.store);
        blocking(Arc::clone(self.jobs.region()), move || {
            let _admission = admission;
            scope.begin_at(&store, &operation, &request, &registered, baseline)
        })
        .await
    }

    #[tracing::instrument(skip_all)]
    #[cfg(test)]
    pub(super) async fn finish_attempt(
        &self,
        operation: String,
        attempt: u8,
        result: ledger::Attempt,
        admission: Arc<WorkAdmission>,
        conditions: Vec<census_domain::model::SourceAccessCondition>,
    ) -> Result<ledger::Completion, JobError> {
        self.finish_attempt_in(
            operation,
            attempt,
            result,
            admission,
            conditions,
            ExecutionLedger::Original,
        )
        .await
    }

    #[tracing::instrument(skip_all)]
    pub(super) async fn finish_attempt_in(
        &self,
        operation: String,
        attempt: u8,
        result: ledger::Attempt,
        admission: Arc<WorkAdmission>,
        conditions: Vec<census_domain::model::SourceAccessCondition>,
        scope: ExecutionLedger,
    ) -> Result<ledger::Completion, JobError> {
        let store = Arc::clone(&self.owner.store);
        blocking(Arc::clone(self.jobs.region()), move || {
            let _admission = admission;
            scope.finish(&store, &operation, attempt, result, &conditions)
        })
        .await
    }
}

#[cfg(test)]
mod tests;
