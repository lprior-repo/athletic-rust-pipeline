//! Model checks for the region ledger.
//!
//! The counters and the task set share one lock; the set's entries are tokio's and cannot enter a
//! model. What is modeled is everything the report promises about *counting*: the same [`Ledger`]
//! the spawner counts through, behind a lock, with workers accepting and finishing units and an
//! observer checking the identity in between.
//!
//! A unit is accepted in one locked step and classified in another, exactly as the spawner does it
//! (accept when the unit is handed to the set, classify when it is reaped), so an observation taken
//! between the two is a state a running region really passes through, not an artificial one.

use loom::sync::{Arc, Mutex};
use loom::thread;

use super::TaskReport;
use super::ledger::Ledger;
use crate::outcome::DrainState;

/// The ledger next to the count of units the region's set still holds — the two numbers the report's
/// identity relates.
#[derive(Default)]
struct Region {
    ledger: Ledger,
    in_flight: u64,
}

#[test]
fn accepted_units_are_counted_exactly_once() {
    let mut builder = loom::model::Builder::new();
    // Two workers, one finishing and one panicking, and the thread budget counts the modeling
    // thread: two workers plus the observer is three. The preemption bound is what keeps the
    // exploration inside the all-features gate's budget.
    builder.max_threads = 3;
    builder.preemption_bound = Some(2);
    builder.check(|| {
        let region = Arc::new(Mutex::new(Region::default()));
        let worker = |state: DrainState| {
            let region = Arc::clone(&region);
            thread::spawn(move || {
                // The locked step the spawner takes when it hands a unit to the set ...
                {
                    let mut held = region.lock().expect("region lock");
                    held.ledger.accept();
                    held.in_flight += 1;
                }
                // ... and the locked step a later reap of that same unit takes.
                {
                    let mut held = region.lock().expect("region lock");
                    held.ledger.classify(state);
                    held.in_flight -= 1;
                }
            })
        };
        let completed = worker(DrainState::Completed);
        let panicked = worker(DrainState::Panicked);

        // The identity holds in every state the region passes through, not only once it is quiet.
        check_identity(&region);
        completed.join().expect("worker finished");
        check_identity(&region);
        panicked.join().expect("worker finished");
        check_identity(&region);

        let mut held = region.lock().expect("region lock");
        assert_eq!(held.in_flight, 0, "every unit finished");
        let report = held.ledger.report();
        assert_eq!(report.accepted, 2);
        assert_eq!(report.completed, 1);
        assert_eq!(report.panicked, 1);
        assert_eq!(report.cancelled, 0);
        assert_eq!(report.timed_out, 0);
        assert_eq!(report.remaining, 0);
        // A drain takes the counters, so the next region starts from zero and the unit that follows
        // is the next drain's business, never this one's.
        held.ledger = Ledger::default();
        assert_eq!(held.ledger.report(), TaskReport::default());
    });
}

/// Every accepted unit is either finished or still in flight — never both, never neither.
fn check_identity(region: &Mutex<Region>) {
    let held = region.lock().expect("region lock");
    let report = held.ledger.report();
    assert_eq!(
        report.accepted,
        report.completed + report.cancelled + report.panicked + report.aborted + held.in_flight,
        "an accepted unit was counted twice, or dropped"
    );
}
