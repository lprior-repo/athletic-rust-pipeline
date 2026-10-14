use std::sync::Arc;

use census_store::Store;
use restate_sdk::prelude::TerminalError;

use super::{ledger, TeamsSource};
use crate::restate_services::wire::TeamsSourceRequest;
use crate::restate_services::JobError;

pub(super) use crate::restate_services::work_admission::WorkAdmission;

pub(super) fn registration_worker(
    admission: Arc<WorkAdmission>,
    store: Arc<Store>,
    operation: String,
    request: Arc<TeamsSourceRequest>,
) -> impl FnOnce() -> Result<ledger::Identity, JobError> + Send + 'static {
    move || {
        let _admission = admission;
        ledger::register(&store, &operation, &request)
    }
}

impl TeamsSource {
    pub(super) async fn admit(&self, key: &str) -> Result<WorkAdmission, TerminalError> {
        self.jobs.admit_work(key).await
    }
}
