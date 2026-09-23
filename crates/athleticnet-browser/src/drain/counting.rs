//! Live unit counters and the guard that closes them.
//!
//! The certificate in [`super`] says what a finished drain did; these counters are how a *running*
//! region answers the same question. The pipeline runtime holds one beside its
//! `TaskTracker`: the tracker says when the pool is empty, these say what the pool did, because
//! `TaskTracker::wait` returns `()`.
//!
//! Every unit is counted exactly once, and the guard is what guarantees it: it is constructed on
//! admission and records the terminal state when it drops, which happens for a unit that returns
//! and for a unit that unwinds alike. Callers declare the guard *after* the tracker token so it
//! drops before the token does, and a drain that sees an empty tracker has therefore already seen
//! every terminal count.

use super::{DrainReport, DrainState};
use std::sync::atomic::{AtomicU64, Ordering};

/// Live counters for units admitted while the runtime is still running.
#[derive(Debug, Default)]
pub struct DrainCounts {
    accepted: AtomicU64,
    completed: AtomicU64,
    cancelled: AtomicU64,
    panicked: AtomicU64,
    open: AtomicU64,
}

impl DrainCounts {
    /// Admit one unit; the returned guard counts its terminal state exactly once.
    pub fn accept(&self) -> DrainUnit<'_> {
        self.accepted.fetch_add(1, Ordering::SeqCst);
        self.open.fetch_add(1, Ordering::SeqCst);
        DrainUnit {
            counts: self,
            finished: false,
        }
    }

    /// The counters as a report, with `open` as `remaining`.
    ///
    /// `timed_out` and `aborted` are the drain's own decisions, so they stay zero here.
    pub fn snapshot(&self) -> DrainReport {
        DrainReport {
            accepted: self.accepted.load(Ordering::SeqCst),
            completed: self.completed.load(Ordering::SeqCst),
            cancelled: self.cancelled.load(Ordering::SeqCst),
            panicked: self.panicked.load(Ordering::SeqCst),
            remaining: self.open.load(Ordering::SeqCst),
            ..DrainReport::default()
        }
    }

    fn record(&self, state: DrainState) {
        let counter = match state {
            DrainState::Completed => &self.completed,
            DrainState::Cancelled => &self.cancelled,
            DrainState::Panicked => &self.panicked,
        };
        counter.fetch_add(1, Ordering::SeqCst);
        self.open.fetch_sub(1, Ordering::SeqCst);
    }
}

/// One admitted unit's accounting guard.
///
/// A unit cancelled before its closure ever started constructs no guard, so it stays uncounted and
/// shows up as `remaining`; that can only happen while the runtime itself is tearing down.
pub struct DrainUnit<'a> {
    counts: &'a DrainCounts,
    finished: bool,
}

impl DrainUnit<'_> {
    /// Mark the unit as having reached a terminal state of its own.
    pub fn complete(&mut self) {
        self.finished = true;
    }
}

impl Drop for DrainUnit<'_> {
    fn drop(&mut self) {
        let state = if self.finished {
            DrainState::Completed
        } else if std::thread::panicking() {
            DrainState::Panicked
        } else {
            DrainState::Cancelled
        };
        self.counts.record(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_finished_unit_is_counted_once_and_a_dropped_one_is_cancelled() {
        let counters = DrainCounts::default();
        {
            let mut unit = counters.accept();
            unit.complete();
        }
        drop(counters.accept());

        let report = counters.snapshot();
        assert_eq!(report.accepted, 2);
        assert_eq!(report.completed, 1);
        assert_eq!(report.cancelled, 1);
        assert_eq!(report.remaining, 0);
    }

    #[test]
    fn a_unit_lost_to_unwinding_is_panicked() {
        let counters = DrainCounts::default();
        std::thread::scope(|scope| {
            let handle = scope.spawn(|| {
                let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _unit = counters.accept();
                    panic!("unit unwinds");
                }));
                assert!(caught.is_err(), "the unit panicked");
            });
            assert!(
                handle.join().is_ok(),
                "the panic was caught inside the thread"
            );
        });

        let report = counters.snapshot();
        assert_eq!(report.accepted, 1);
        assert_eq!(report.panicked, 1);
        assert_eq!(report.completed, 0);
        assert_eq!(report.remaining, 0);
    }
}
