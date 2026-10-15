use crate::spawn::{
    Reservation, Spawner, WORK_CLASS_CONTROL, WORK_CLASS_PUBLICATION, WORK_CLASS_STAGE,
};
use census_store::Store;
use restate_sdk::prelude::TerminalError;
use std::sync::Arc;

#[derive(Clone)]
pub struct Jobs {
    pub(super) store: Arc<Store>,
    pub(super) region: Arc<Spawner>,
    gates: crate::restate_services::work_admission::WorkGates,
}

impl Jobs {
    pub fn new(store: Arc<Store>, region: Arc<Spawner>) -> Self {
        Self {
            store,
            region,
            gates: crate::restate_services::work_admission::WorkGates::default(),
        }
    }

    pub(in crate::restate_services) async fn admit_work(
        &self,
        key: &str,
    ) -> Result<crate::restate_services::work_admission::WorkAdmission, TerminalError> {
        self.gates.admit(self, key).await
    }

    pub(in crate::restate_services) fn store(&self) -> &Arc<Store> {
        &self.store
    }

    pub(in crate::restate_services) fn region(&self) -> &Arc<Spawner> {
        &self.region
    }

    pub(in crate::restate_services) async fn permit(&self) -> Result<Reservation, TerminalError> {
        self.region
            .coordinator()
            .reserve(WORK_CLASS_STAGE)
            .await
            .map_err(|error| TerminalError::new(format!("stage admission: {error}")))
    }

    pub(in crate::restate_services) async fn permit_publication(
        &self,
    ) -> Result<Reservation, TerminalError> {
        self.region
            .coordinator()
            .reserve(WORK_CLASS_PUBLICATION)
            .await
            .map_err(|error| TerminalError::new(format!("publication admission: {error}")))
    }

    pub(in crate::restate_services) async fn permit_control(
        &self,
    ) -> Result<Reservation, TerminalError> {
        self.region
            .coordinator()
            .reserve(WORK_CLASS_CONTROL)
            .await
            .map_err(|error| TerminalError::new(format!("control admission: {error}")))
    }
}

pub(super) async fn run_reserved<T, E, F>(
    region: Arc<Spawner>,
    permit: Reservation,
    job: F,
) -> Result<T, super::JobError>
where
    F: FnOnce() -> Result<T, E> + Send + 'static,
    T: Send + 'static,
    E: Into<super::JobError> + Send + 'static,
{
    super::blocking(region, move || {
        let _permit = permit;
        job()
    })
    .await
}

#[cfg(test)]
mod publication_tests;
