use super::Actor;
use crate::drain::{count, DrainAccountingError};

impl Actor {
    pub(crate) fn record_accounting(&mut self, result: Result<(), DrainAccountingError>) {
        if let Err(error) = result {
            tracing::error!(%error, "browser drain accounting failed; cleanup remains owned");
            self.accounting_error = Some(error);
            self.draining = true;
            self.shutdown.cancel();
            self.observer_stop.cancel();
            self.gate.revoke();
        }
    }

    pub(crate) fn accept_drain_tasks(&mut self, len: usize) {
        let result = count(len).and_then(|count| self.region.accept(count));
        self.record_accounting(result);
    }
}
