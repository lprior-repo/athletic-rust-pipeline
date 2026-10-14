use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use tokio::task::{Id, JoinError};

use crate::outcome::DrainState;

use super::TaskReport;

#[derive(Default)]
pub(super) struct Counters {
    started: AtomicBool,
    failed: AtomicBool,
}

impl Counters {
    pub(super) fn record_started(&self) {
        self.started.store(true, Ordering::Release);
    }

    pub(super) fn record_failed(&self) {
        self.failed.store(true, Ordering::Release);
    }
}

#[derive(Default)]
pub(super) struct Ledger {
    report: TaskReport,
    blocking: BTreeMap<Id, Arc<Counters>>,
}

impl Ledger {
    pub(super) fn holding(held: u64) -> Self {
        Self {
            report: TaskReport {
                accepted: held,
                ..TaskReport::default()
            },
            ..Self::default()
        }
    }

    pub(super) fn track_blocking(&mut self, id: Id, counters: Arc<Counters>) {
        self.blocking.insert(id, counters);
    }

    pub(super) fn tracked_blocking_count(&self) -> usize {
        self.blocking.len()
    }

    pub(super) fn accept(&mut self) {
        bump(&mut self.report.accepted, 1);
    }

    pub(super) fn classify_join(&mut self, joined: Result<(Id, ()), JoinError>) {
        let (id, state) = match joined {
            Ok((id, ())) => (id, DrainState::Completed),
            Err(error) => (error.id(), DrainState::from_join(Err(error))),
        };
        let witness = self.blocking.remove(&id);
        if state == DrainState::Cancelled
            && witness
                .as_ref()
                .is_none_or(|task| task.started.load(Ordering::Acquire))
        {
            bump(&mut self.report.aborted, 1);
        } else {
            self.classify(state);
        }
        if witness.is_some_and(|task| task.failed.load(Ordering::Acquire)) {
            bump(&mut self.report.failed, 1);
        }
    }

    pub(super) fn classify_thread(&mut self, joined: std::thread::Result<()>, witness: &Counters) {
        self.classify(match joined {
            Ok(()) => DrainState::Completed,
            Err(_) => DrainState::Panicked,
        });
        if witness.failed.load(Ordering::Acquire) {
            bump(&mut self.report.failed, 1);
        }
    }

    pub(super) fn classify(&mut self, state: DrainState) {
        match state {
            DrainState::Completed => bump(&mut self.report.completed, 1),
            DrainState::Panicked => {
                tracing::error!("a region task panicked");
                bump(&mut self.report.panicked, 1);
            }
            DrainState::Cancelled => bump(&mut self.report.cancelled, 1),
        }
    }

    pub(super) fn note_deadline(&mut self, remaining: u64) {
        self.set_remaining(remaining);
        bump(&mut self.report.timed_out, remaining);
    }

    pub(super) fn set_remaining(&mut self, remaining: u64) {
        self.report.remaining = remaining;
        self.report.still_running = remaining;
    }

    pub(super) fn report(&self) -> TaskReport {
        self.report
    }
}

fn bump(counter: &mut u64, by: u64) {
    *counter = counter.saturating_add(by);
}
