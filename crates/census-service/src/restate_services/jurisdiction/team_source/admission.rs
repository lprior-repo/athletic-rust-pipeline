use std::sync::Arc;

use census_store::Store;
use restate_sdk::prelude::TerminalError;
use tokio::sync::{Mutex, OwnedMutexGuard, OwnedSemaphorePermit};

use super::{ledger, TeamsSource};
use crate::restate_services::wire::TeamsSourceRequest;
use crate::restate_services::JobError;

pub(super) struct WorkAdmission {
    _work: OwnedSemaphorePermit,
    _source: OwnedMutexGuard<()>,
}

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
        let work = self.jobs.permit().await?;
        let source = {
            let mut gates = self.gates.lock().await;
            gates.retain(|_, gate| gate.strong_count() > 0);
            match gates.get(key).and_then(std::sync::Weak::upgrade) {
                Some(gate) => gate,
                None => {
                    let gate = Arc::new(Mutex::new(()));
                    gates.insert(key.to_string(), Arc::downgrade(&gate));
                    gate
                }
            }
        };
        Ok(WorkAdmission {
            _work: work,
            _source: source.lock_owned().await,
        })
    }
}
