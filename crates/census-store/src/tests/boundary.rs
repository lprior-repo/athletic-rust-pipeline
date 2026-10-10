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
        let (rows, evidence, derived) = receiver.recv_timeout(std::time::Duration::from_secs(60))?;
        check!(eq; rows?, 1);
        check!(eq; evidence, store.evidence_generation());
        check!(eq; derived, store.derived_generation());
        Ok(())
    })
}
