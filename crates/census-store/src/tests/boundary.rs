use super::*;

#[test]
fn a_snapshot_cannot_pin_its_view_between_an_append_and_its_generations() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.append(Table::Schools, &school("Boundary High School"))?;
    let held = store.lock_appends();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::scope(|scope| -> TestResult {
        scope.spawn(|| {
            let snapshot = store.snapshot();
            let _ = sender.send((
                snapshot
                    .scan::<CanonicalSchool>(Table::Schools)
                    .map(|rows| rows.len()),
                snapshot.evidence_generation(),
                snapshot.derived_generation(),
            ));
        });
        check!(
            receiver.recv_timeout(std::time::Duration::from_secs(2)).is_err(),
            "a snapshot completed while another thread held the append lock, so its view and generations can cross a commit"
        );
        drop(held);
        let (rows, evidence, derived) =
            receiver.recv_timeout(std::time::Duration::from_secs(60))?;
        check!(eq; rows?, 1);
        check!(eq; evidence, store.evidence_generation());
        check!(eq; derived, store.derived_generation());
        Ok(())
    })
}

#[test]
fn a_flush_does_not_change_what_scan_returns() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let schools = vec![school("Flush Visibility A"), school("Flush Visibility B")];
    store.append_many(Table::Schools, &schools)?;
    let scanned = store.scan::<CanonicalSchool>(Table::Schools)?;
    store.flush()?;
    let rescanned = store.scan::<CanonicalSchool>(Table::Schools)?;
    check!(eq; scanned.len(), 2);
    check!(
        eq;
        rescanned,
        scanned,
        "a flush is a durability barrier, not a read boundary"
    );
    Ok(())
}

#[test]
fn scan_sees_each_committed_batch_whole_and_never_its_halves() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let standing = school("Standing School");
    store.append(Table::Schools, &standing)?;
    let pinned = store.snapshot();
    let added = vec![school("Added School 0"), school("Added School 1")];
    store.append_many(Table::Schools, &added)?;
    check!(
        eq;
        pinned.scan::<CanonicalSchool>(Table::Schools)?,
        vec![standing.clone()],
        "a pinned snapshot cannot observe a batch that committed after it"
    );
    let current = store.scan::<CanonicalSchool>(Table::Schools)?;
    check!(eq; current.len(), 3, "the committed batch is visible in whole");
    check!(current.iter().any(|row| row.name == "Standing School"));
    check!(current.iter().any(|row| row.name == "Added School 0"));
    check!(current.iter().any(|row| row.name == "Added School 1"));
    Ok(())
}
