//! The region's counters: what each unit's fate adds up to.
//!
//! Kept apart from the task set so the arithmetic can be checked on its own. The set's entries are
//! tokio's and cannot enter a model check, but a report's promise is arithmetic: every accepted unit
//! is counted exactly once, and a drain's deadline moves exactly the units it found still in
//! flight. `spawn/loom_tests.rs` runs this ledger behind a lock over the schedules `loom` explores.

use crate::outcome::DrainState;

use super::TaskReport;

/// What a region has counted so far.
///
/// One lock holds a ledger and the set it describes (see `super::Region`), so the two can never
/// disagree, and counts saturate: a counter at its ceiling stays there instead of wrapping into a
/// smaller, quieter number.
#[derive(Default)]
pub(super) struct Ledger {
    report: TaskReport,
}

impl Ledger {
    /// A ledger that already accounts for `held` units the region adopted.
    pub(super) fn holding(held: u64) -> Self {
        let report = TaskReport {
            accepted: held,
            ..TaskReport::default()
        };
        Self { report }
    }

    /// Count one unit the region accepted.
    pub(super) fn accept(&mut self) {
        bump(&mut self.report.accepted, 1);
    }

    /// Count one unit that finished inside a drain's deadline.
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

    /// Count one unit the drain reclaimed after the deadline.
    ///
    /// The one difference from [`Ledger::classify`]: a cancellation reaped here is the abort this
    /// drain just issued, which the report calls `aborted`, not a cancellation that arrived on its
    /// own.
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

    /// Record what a drain's deadline found still in flight.
    ///
    /// `timed_out` carries the count at the deadline — what the abort that follows is about to
    /// reclaim. `remaining` is set to the same value here but may be updated after reaping
    /// with [`Ledger::set_remaining`]; only then does it reflect what the abort could not reclaim.
    pub(super) fn note_deadline(&mut self, remaining: u64) {
        self.report.remaining = remaining;
        bump(&mut self.report.timed_out, remaining);
    }

    /// Update `remaining` after the reaping phase.
    ///
    /// Called after `try_join_next` has drained everything that completed in response to the abort;
    /// `timed_out` stays as set by [`Ledger::note_deadline`].
    pub(super) fn set_remaining(&mut self, remaining: u64) {
        self.report.remaining = remaining;
    }

    /// The counters as they stand.
    pub(super) fn report(&self) -> TaskReport {
        self.report
    }
}

/// Add to a counter without wrapping.
fn bump(counter: &mut u64, by: u64) {
    *counter = counter.saturating_add(by);
}
