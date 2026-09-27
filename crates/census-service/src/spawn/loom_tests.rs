use loom::sync::{Arc, Mutex};
use loom::thread;

use super::ledger::Ledger;
use super::TaskReport;
use crate::outcome::DrainState;

#[derive(Default)]
struct Region {
    ledger: Ledger,
    in_flight: u64,
}

#[test]
fn accepted_units_are_counted_exactly_once() {
    let mut builder = loom::model::Builder::new();
    builder.max_threads = 3;
    builder.preemption_bound = Some(2);
    builder.check(|| {
        let region = Arc::new(Mutex::new(Region::default()));
        let worker = |state: DrainState| {
            let region = Arc::clone(&region);
            thread::spawn(move || {
                {
                    let mut held = region.lock().expect("region lock");
                    held.ledger.accept();
                    held.in_flight += 1;
                }
                {
                    let mut held = region.lock().expect("region lock");
                    held.ledger.classify(state);
                    held.in_flight -= 1;
                }
            })
        };
        let completed = worker(DrainState::Completed);
        let panicked = worker(DrainState::Panicked);

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
        held.ledger = Ledger::default();
        assert_eq!(held.ledger.report(), TaskReport::default());
    });
}

fn check_identity(region: &Mutex<Region>) {
    let held = region.lock().expect("region lock");
    let report = held.ledger.report();
    assert_eq!(
        report.accepted,
        report.completed + report.cancelled + report.panicked + report.aborted + held.in_flight,
        "an accepted unit was counted twice, or dropped"
    );
}
