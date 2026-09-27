
use std::sync::Mutex;
use tokio::sync::Notify;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GateSnapshot {
    pub(crate) generation: u64,
    pub(crate) ready: bool,
}

struct GateState {
    generation: u64,
    ready: bool,
}

pub(crate) struct ProfileGate {
    state: Mutex<GateState>,
    notify: Notify,
}

impl ProfileGate {
    pub(crate) fn new() -> Self {
        Self {
            state: Mutex::new(GateState {
                generation: 0,
                ready: false,
            }),
            notify: Notify::new(),
        }
    }

    pub(crate) fn snapshot(&self) -> GateSnapshot {
        match self.state.lock() {
            Ok(guard) => GateSnapshot {
                generation: guard.generation,
                ready: guard.ready,
            },
            Err(_) => GateSnapshot {
                generation: u64::MAX,
                ready: false,
            },
        }
    }

    pub(crate) fn revoke(&self) {
        if let Ok(mut guard) = self.state.lock() {
            guard.generation = guard.generation.saturating_add(1);
            guard.ready = false;
        }
        self.notify.notify_one();
    }

    pub(crate) fn try_open(&self, observed_generation: u64) -> bool {
        match self.state.lock() {
            Ok(mut guard) => {
                if guard.generation == u64::MAX {
                    return false;
                }
                if guard.generation != observed_generation {
                    return false;
                }
                guard.ready = true;
                true
            }
            Err(_) => false,
        }
    }

    pub(crate) async fn closed(&self) {
        loop {
            let notified = self.notify.notified();
            let snap = self.snapshot();
            if !snap.ready {
                return;
            }
            notified.await;
        }
    }
    pub(crate) fn is_ready(&self) -> bool {
        match self.state.lock() {
            Ok(guard) => guard.ready,
            Err(_) => false,
        }
    }
}
