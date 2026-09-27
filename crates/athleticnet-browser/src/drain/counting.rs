
use super::{DrainReport, DrainState};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Default)]
pub struct DrainCounts {
    accepted: AtomicU64,
    completed: AtomicU64,
    cancelled: AtomicU64,
    panicked: AtomicU64,
    open: AtomicU64,
}

impl DrainCounts {
    pub fn accept(&self) -> DrainUnit<'_> {
        self.accepted.fetch_add(1, Ordering::SeqCst);
        self.open.fetch_add(1, Ordering::SeqCst);
        DrainUnit {
            counts: self,
            finished: false,
        }
    }

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

pub struct DrainUnit<'a> {
    counts: &'a DrainCounts,
    finished: bool,
}

impl DrainUnit<'_> {
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
