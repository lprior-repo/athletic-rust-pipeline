use loom::sync::{Arc, Mutex};
use loom::thread;

use super::ledger::Ledger;
use super::TaskReport;
use crate::outcome::DrainState;

type TestResult = Result<(), Box<dyn std::error::Error>>;

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
        let result = run_permutation();
        assert!(result.is_ok(), "permutation failed: {result:?}");
    });
}

fn run_permutation() -> TestResult {
    let region = Arc::new(Mutex::new(Region::default()));
    let worker = |state: DrainState| {
        let region = Arc::clone(&region);
        thread::spawn(move || -> Result<(), &'static str> {
            {
                let mut held = region.lock().map_err(|_| "region lock poisoned")?;
                held.ledger.accept();
                held.in_flight += 1;
            }
            {
                let mut held = region.lock().map_err(|_| "region lock poisoned")?;
                held.ledger.classify(state);
                held.in_flight -= 1;
            }
            Ok(())
        })
    };
    let completed = worker(DrainState::Completed);
    let panicked = worker(DrainState::Panicked);

    check_identity(&region)?;
    completed
        .join()
        .map_err(|_| "completed worker panicked")??;
    check_identity(&region)?;
    panicked
        .join()
        .map_err(|_| "classified worker panicked")??;
    check_identity(&region)?;

    let mut held = region.lock().map_err(|_| "region lock poisoned")?;
    check!(eq; held.in_flight, 0, "every unit finished");
    let report = held.ledger.report();
    check!(eq; report.accepted, 2);
    check!(eq; report.completed, 1);
    check!(eq; report.panicked, 1);
    check!(eq; report.cancelled, 0);
    check!(eq; report.timed_out, 0);
    check!(eq; report.remaining, 0);
    held.ledger = Ledger::default();
    check!(eq; held.ledger.report(), TaskReport::default());
    Ok(())
}

fn check_identity(region: &Mutex<Region>) -> TestResult {
    let held = region.lock().map_err(|_| "region lock poisoned")?;
    let report = held.ledger.report();
    check!(eq;
        report.accepted,
        report.completed + report.cancelled + report.panicked + report.aborted + held.in_flight,
        "an accepted unit was counted twice, or dropped"
    );
    Ok(())
}
