use super::*;
#[test]
fn registration_refuses_wrong_object_keys_without_mutation() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut request = request()?;
    let operation = key(&request);
    let store = Store::open(root.path())?;
    let before = journal(&store)?;
    check!(matches!(
        ledger::register(&store, "wrong-object-key", &request),
        Err(JobError::Terminal { .. })
    ));
    check!(eq; journal(&store)?, before);
    ledger::register(&store, &operation, &request)?;
    let before = journal(&store)?;
    request.source = "coach_directories".to_string();
    check!(matches!(
        ledger::register(&store, &operation, &request),
        Err(JobError::Terminal { .. })
    ));
    check!(eq; journal(&store)?, before);
    Ok(())
}

#[test]
fn registration_refuses_corrupted_identity_without_mutation() -> TestResult {
    for payload in [
        json!({"request_digest":"different","observed_on":"2026-10-02"}),
        json!({"request_digest":17,"observed_on":"2026-10-02"}),
        json!({"request_digest":"different"}),
    ] {
        let root = tempfile::tempdir()?;
        let request = request()?;
        let operation = key(&request);
        let store = Store::open(root.path())?;
        let mut batch = store.write_batch();
        batch.journal_done(ledger::PHASE, &format!("{operation}/identity"), &payload)?;
        batch.commit()?;
        let before = journal(&store)?;
        check!(matches!(
            ledger::register(&store, &operation, &request),
            Err(JobError::Terminal { .. })
        ));
        check!(eq; journal(&store)?, before);
    }
    Ok(())
}

#[test]
fn registration_refuses_orphan_history_without_creating_identity() -> TestResult {
    for (suffix, payload) in [
        (
            "attempt/1/reserved",
            json!({"attempt": 1, "observed_on": "2026-10-02"}),
        ),
        (
            "attempt/1/outcome",
            json!({"status": "transient", "message": "orphan", "progress": null}),
        ),
        (
            "settled",
            json!({"status": "terminal", "message": "orphan", "progress": []}),
        ),
    ] {
        let root = tempfile::tempdir()?;
        let request = request()?;
        let operation = key(&request);
        let store = Store::open(root.path())?;

        let mut batch = store.write_batch();
        batch.journal_done(ledger::PHASE, &format!("{operation}/{suffix}"), &payload)?;
        batch.commit()?;

        let before = journal(&store)?;

        let result = ledger::register(&store, &operation, &request);
        check!(matches!(result, Err(JobError::Terminal { .. })));

        let after = journal(&store)?;
        check!(eq; after, before);

        let identity = store.journal_payload(ledger::PHASE, &format!("{operation}/identity"))?;
        check!(eq; identity, None);
    }
    Ok(())
}
#[test]
fn admission_refuses_cached_registration_when_fjall_identity_is_missing() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    let operation = key(&request);
    let store = Store::open(root.path())?;
    let registered = ledger::register(&store, &operation, &request)?;

    let missing_root = tempfile::tempdir()?;
    let missing_store = Store::open(missing_root.path())?;
    let before = journal(&missing_store)?;

    let result = ledger::begin(&missing_store, &operation, &request, &registered);
    check!(matches!(result, Err(JobError::Terminal { .. })));
    check!(eq; journal(&missing_store)?, before);

    Ok(())
}

#[test]
fn registration_refuses_incoherent_existing_reservation_without_mutation() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    let operation = key(&request);
    let store = Store::open(root.path())?;
    ledger::register(&store, &operation, &request)?;

    let mut batch = store.write_batch();
    batch.journal_done(
        ledger::PHASE,
        &format!("{operation}/attempt/1/reserved"),
        &json!({"attempt": 1, "observed_on": "2026-10-03"}),
    )?;
    batch.commit()?;

    let before = journal(&store)?;
    let result = ledger::register(&store, &operation, &request);
    check!(matches!(result, Err(JobError::Terminal { .. })));
    check!(eq; journal(&store)?, before);

    Ok(())
}
