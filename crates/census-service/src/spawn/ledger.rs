
use crate::outcome::DrainState;

use super::TaskReport;

#[derive(Default)]
pub(super) struct Ledger {
    report: TaskReport,
}

impl Ledger {
    pub(super) fn holding(held: u64) -> Self {
        let report = TaskReport {
            accepted: held,
            ..TaskReport::default()
        };
        Self { report }
    }

    pub(super) fn accept(&mut self) {
        bump(&mut self.report.accepted, 1);
    }

    pub(super) fn classify(&mut self, state: DrainState) {
        match state {
            DrainState::Completed => bump(&mut self.report.completed, 1),
            DrainState::Cancelled => bump(&mut self.report.cancelled, 1),
            DrainState::Panicked => {
                tracing::error!("a region task panicked");
                bump(&mut self.report.panicked, 1);
            }
        }
    }

    pub(super) fn classify_reaped(&mut self, state: DrainState) {
        match state {
            DrainState::Completed => bump(&mut self.report.completed, 1),
            DrainState::Panicked => {
                tracing::error!("a region task panicked");
                bump(&mut self.report.panicked, 1);
            }
            DrainState::Cancelled => bump(&mut self.report.aborted, 1),
        }
    }

    pub(super) fn note_deadline(&mut self, remaining: u64) {
        self.report.remaining = remaining;
        bump(&mut self.report.timed_out, remaining);
    }

    pub(super) fn set_remaining(&mut self, remaining: u64) {
        self.report.remaining = remaining;
    }

    pub(super) fn report(&self) -> TaskReport {
        self.report
    }
}

fn bump(counter: &mut u64, by: u64) {
    *counter = counter.saturating_add(by);
}
