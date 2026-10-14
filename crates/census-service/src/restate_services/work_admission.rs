use super::Jobs;
use crate::spawn::Reservation;
use restate_sdk::prelude::TerminalError;
use std::collections::HashMap;
use std::sync::{Arc, Weak};
use tokio::sync::{Mutex, OwnedMutexGuard};

#[derive(Clone, Default)]
pub(super) struct WorkGates(Arc<Mutex<HashMap<String, Weak<Mutex<()>>>>>);

pub(crate) struct WorkAdmission {
    _work: Reservation,
    _source: OwnedMutexGuard<()>,
}

impl WorkGates {
    pub(super) async fn admit(
        &self,
        jobs: &Jobs,
        key: &str,
    ) -> Result<WorkAdmission, TerminalError> {
        let work = jobs.permit().await?;
        let source = {
            let mut gates = self.0.lock().await;
            gates.retain(|_, gate| gate.strong_count() > 0);
            match gates.get(key).and_then(Weak::upgrade) {
                Some(gate) => gate,
                None => {
                    gates
                        .try_reserve(1)
                        .map_err(|_| TerminalError::new("work gate allocation failed"))?;
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

#[cfg(test)]
mod tests;
