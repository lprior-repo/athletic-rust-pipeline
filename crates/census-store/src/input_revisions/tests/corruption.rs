use super::*;

fn raw_capability(store: &Store, raw: &[u8]) -> TestResult {
    store.meta.insert(KEY, raw)?;
    store.flush()?;
    Ok(())
}

#[test]
fn malformed_partial_unknown_and_oversized_records_refuse_without_repair() -> TestResult {
    let malformed = [
        b"{}".to_vec(),
        b"null".to_vec(),
        b"\xff".to_vec(),
        br#"{"version":2,"baseline_evidence_generation":0,"baseline_derived_generation":0,"counters":[0,0,0,0,0]}"#.to_vec(),
        br#"{"version":3,"baseline_evidence_generation":0,"baseline_derived_generation":0,"counters":[0,0,0,0,0,0]}"#.to_vec(),
        br#"{"version":2,"baseline_evidence_generation":0,"baseline_derived_generation":0,"counters":[-1,0,0,0,0,0]}"#.to_vec(),
        br#"{"version":2,"baseline_evidence_generation":0,"baseline_derived_generation":0,"counters":[18446744073709551616,0,0,0,0,0]}"#.to_vec(),
        br#"{"version":2,"baseline_evidence_generation":0,"baseline_derived_generation":0,"counters":[0,0,0,0,0,0],"unknown":0}"#.to_vec(),
        br#"{"version":2,"version":2,"baseline_evidence_generation":0,"baseline_derived_generation":0,"counters":[0,0,0,0,0,0]}"#.to_vec(),
        vec![b'x'; 513],
    ];
    for raw in malformed {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        raw_capability(&store, &raw)?;
        let before = retained_state(&store)?;
        check!(store
            .snapshot()
            .public_identity_input_revisions_v2()
            .is_err());
        check!(store
            .enroll_public_identity_input_revisions_v2(store.snapshot().revision())
            .is_err());
        check!(store
            .append(Table::Athletes, &derived("refused", 1))
            .is_err());
        check!(eq; retained_state(&store)?, before);
        drop(store);
        check!(Store::open_existing(directory.path()).is_err());
        check!(Store::migrate(directory.path()).is_err());
    }
    Ok(())
}

#[test]
fn unknown_or_extra_capability_keys_are_not_disabled_or_ignored() -> TestResult {
    for enrolled in [false, true] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        if enrolled {
            enroll(&store)?;
        }
        store.meta.insert(format!("{PREFIX}v3"), b"{}")?;
        let before = retained_state(&store)?;
        check!(store
            .snapshot()
            .public_identity_input_revisions_v2()
            .is_err());
        check!(store
            .enroll_public_identity_input_revisions_v2(store.snapshot().revision())
            .is_err());
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}

#[test]
fn future_enrollment_baseline_is_refused_without_repair() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    raw_capability(&store, br#"{"version":2,"baseline_evidence_generation":1,"baseline_derived_generation":0,"counters":[0,0,0,0,0,0]}"#)?;
    let before = retained_state(&store)?;
    check!(store
        .snapshot()
        .public_identity_input_revisions_v2()
        .is_err());
    check!(store
        .enroll_public_identity_input_revisions_v2(store.snapshot().revision())
        .is_err());
    check!(eq; retained_state(&store)?, before);
    drop(store);
    check!(Store::open_existing(directory.path()).is_err());
    check!(Store::migrate(directory.path()).is_err());
    Ok(())
}

#[test]
fn capability_cannot_enter_legacy_migration_after_a_schema_downgrade() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    enroll(&store)?;
    store.meta.insert("schema:store_version", b"1")?;
    store.flush()?;
    drop(store);
    check!(Store::open_existing(directory.path()).is_err());
    check!(Store::migrate(directory.path()).is_err());
    let db = fjall::Database::builder(directory.path().join(crate::DB_DIR)).open()?;
    let meta = db.keyspace(crate::META, fjall::KeyspaceCreateOptions::default)?;
    check!(eq; crate::meta::get_text(&meta, "schema:store_version")?, Some("1".to_owned()));
    check!(eq; crate::meta::get_text(&meta, crate::format::MIGRATING_TO)?, None);
    Ok(())
}

#[test]
fn corrupt_capability_cannot_replace_a_valid_backup_generation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("source");
    let backup = directory.path().join("backup");
    let restored = directory.path().join("restored");
    let store = Store::open(&root)?;
    enroll(&store)?;
    mutate(&store, Table::Athletes, "retained")?;
    let token = enabled(&store)?;
    drop(store);
    Store::backup(&root, &backup)?;
    let manifest = std::fs::read(backup.join("backup.json"))?;
    Store::restore(&backup, &restored)?;
    let restored_store = Store::open_existing(&restored)?;
    check!(eq; enabled(&restored_store)?, token);
    drop(restored_store);
    let store = Store::open_existing(&root)?;
    raw_capability(&store, b"{}")?;
    drop(store);
    check!(Store::backup(&root, &backup).is_err());
    check!(eq; std::fs::read(backup.join("backup.json"))?, manifest);
    Ok(())
}
