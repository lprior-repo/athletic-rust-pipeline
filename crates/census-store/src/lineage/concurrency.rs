use std::sync::{mpsc, Arc, Barrier, TryLockError};
use std::time::Duration;

use crate::Store;

use super::process::bounded_case;
use super::support::{canonical, equal, fixture_dir, lineage_rows, seed, TestResult, Witness};

#[test]
fn barrier_started_clones_share_one_durable_identity_and_one_sequence_transition() -> TestResult {
    bounded_case(
        concat!(
            module_path!(),
            "::barrier_started_clones_share_one_durable_identity_and_one_sequence_transition"
        ),
        || {
            let dir = fixture_dir()?;
            let store = Store::open_with(dir.path(), 8_388_608)?;
            seed(&store)?;
            let before = Witness::capture(&store)?;
            let start = Arc::new(Barrier::new(9));
            let identities = std::thread::scope(|scope| -> TestResult<Vec<_>> {
                let mut workers = Vec::with_capacity(8);
                for _ in 0..8 {
                    let clone = store.clone();
                    let start = Arc::clone(&start);
                    workers.push(scope.spawn(move || {
                        start.wait();
                        clone.store_identity()
                    }));
                }
                start.wait();
                let mut identities = Vec::with_capacity(8);
                let mut failures = Vec::new();
                for worker in workers {
                    match worker.join() {
                        Ok(Ok(identity)) => identities.push(identity),
                        Ok(Err(error)) => failures.push(format!("clone refused identity: {error}")),
                        Err(_) => failures.push("clone identity worker panicked".to_owned()),
                    }
                }
                if !failures.is_empty() {
                    return Err(failures.join("; ").into());
                }
                Ok(identities)
            })?;
            equal(identities.len(), 8)?;
            let identity = identities
                .first()
                .ok_or("no clone produced an identity")?
                .clone();
            for observed in &identities {
                equal(observed, &identity)?;
            }
            let after = Witness::capture(&store)?;
            before.unchanged_evidence(&after)?;
            equal(
                after.sequence,
                before.sequence.checked_add(1).ok_or("sequence overflow")?,
            )?;
            let records = lineage_rows(&store)?;
            equal(records.len(), 1)?;
            let (_, raw) = records
                .first()
                .ok_or("the winning journal record is missing")?;
            equal(records.clone(), vec![canonical(raw)])?;
            store.flush()?;
            drop(store);
            let reopened = Store::open_with(dir.path(), 8_388_608)?;
            equal(reopened.store_identity()?, identity)?;
            equal(lineage_rows(&reopened)?, records)?;
            equal(Witness::capture(&reopened)?, after)
        },
    )
}

#[test]
fn live_identity_ensure_waits_for_the_shared_appends_boundary_and_drains_after_release(
) -> TestResult {
    bounded_case(
        concat!(
            module_path!(),
            "::live_identity_ensure_waits_for_the_shared_appends_boundary_and_drains_after_release"
        ),
        appends_boundary_case,
    )
}

fn appends_boundary_case() -> TestResult {
    let dir = fixture_dir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let before = Witness::capture(&store)?;
    let held = store.lock_appends();
    let clone = store.clone();
    let (ready_tx, ready_rx) = mpsc::sync_channel(1);
    let (done_tx, done_rx) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || -> TestResult {
        let observed = match clone.appends.try_lock() {
            Err(TryLockError::WouldBlock) => Ok(()),
            Err(TryLockError::Poisoned(_)) => Err("the appends lock was poisoned".to_owned()),
            Ok(guard) => {
                drop(guard);
                Err("the clone did not share the held appends lock".to_owned())
            }
        };
        ready_tx.send(observed)?;
        done_tx.send(clone.store_identity())?;
        Ok(())
    });
    let started = ready_rx.recv_timeout(Duration::from_secs(5));
    let premature = done_rx.recv_timeout(Duration::from_millis(200));
    let blocked = matches!(&premature, Err(mpsc::RecvTimeoutError::Timeout));
    let locked_sequence = store.db.snapshot().seqno();
    drop(held);
    let completed = match premature {
        Ok(result) => Ok(result),
        Err(mpsc::RecvTimeoutError::Timeout) => done_rx.recv_timeout(Duration::from_secs(5)),
        Err(error) => Err(error),
    };
    let joined = worker.join();
    joined.map_err(|_| "the appends-boundary worker panicked")??;
    started??;
    let identity = completed??;
    if !blocked {
        return Err("identity ensure completed before the held appends guard was released".into());
    }
    equal(locked_sequence, before.sequence)?;
    let after = Witness::capture(&store)?;
    before.unchanged_evidence(&after)?;
    equal(
        after.sequence,
        before.sequence.checked_add(1).ok_or("sequence overflow")?,
    )?;
    equal(store.snapshot().store_identity()?, identity)?;
    equal(lineage_rows(&store)?.len(), 1)
}
