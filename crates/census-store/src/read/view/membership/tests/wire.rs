use super::*;

fn chunked(store: &Store) -> TestResult {
    let encoded = br#"{"key":"row/a","payload":{"revision":1}}"#;
    raw(store, "row/a", &manifest_value("row/a", 1, encoded.len())?)?;
    let key = chunk_key(&Store::journal_key(PHASE, "row/a"), 0)?;
    store
        .journal
        .insert(key, chunk_value("row/a", 0, 1, encoded)?)?;
    Ok(())
}

#[test]
fn physical_chunks_are_charged_but_only_the_logical_root_is_emitted() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    chunked(&store)?;
    let view = store.snapshot();
    let mut seen = Vec::new();
    let inventory = view.for_each_journal_root_bounded(namespace()?, budget(2), |key| {
        seen.push(key.to_string());
        Ok(())
    })?;
    check!(eq; seen, vec!["row/a".to_string()]);
    check!(eq; inventory.logical_roots(), 1);
    check!(eq; inventory.fingerprint().physical_rows(), 2);
    check!(eq; view.journal_payload(PHASE, "row/a")?, Some(json!({"revision": 1})));
    check!(matches!(
        view.for_each_journal_root_bounded(namespace()?, budget(1), |_| Ok(())),
        Err(StoreError::BudgetExceeded {
            what: "journal physical rows",
            needed: 2,
            available: 1
        })
    ));
    Ok(())
}

#[test]
fn source_owned_chunk_names_remain_logical_roots() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    put(&store, PHASE, "row/capture/chunk/00000000", 1)?;
    put(&store, PHASE, "row/capture/manifest", 1)?;
    let inventory = roots(&store.snapshot())?;
    check!(eq; inventory.logical_roots(), 2);
    check!(eq; inventory.fingerprint().physical_rows(), 2);
    Ok(())
}

#[test]
fn malformed_json_and_null_type_refuse_with_the_root_decode_category() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let cases: &[(&[u8], serde_json::error::Category)] = &[
        (b"{", serde_json::error::Category::Eof),
        (b"!", serde_json::error::Category::Syntax),
        (
            br#"{"key":"row/a","payload":null,"type":null}"#,
            serde_json::error::Category::Data,
        ),
    ];
    for (value, category) in cases {
        raw(&store, "row/a", value)?;
        check!(
            matches!(roots(&store.snapshot()), Err(StoreError::Decode { key, source })
            if key == format!("journal root type for {PHASE}/row/a") && source.classify() == *category)
        );
    }
    Ok(())
}

#[test]
fn foreign_and_unsupported_envelopes_refuse_with_exact_owned_paths() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let cases: &[(&[u8], &str)] = &[
        (br#"{"key":"row/foreign","payload":null}"#, "entry key does not match its lookup"),
        (br#"{"key":"row/a","payload":null,"version":99}"#, "entry contains an unsupported type or version"),
        (br#"{"type":"future-root","version":9}"#, "root is neither an inline entry nor a chunked-manifest"),
        (br#"{"type":"chunked-manifest","version":99,"key":"row/a","chunks":1,"total_bytes":1}"#, "manifest is not version 1"),
        (br#"{"type":"chunked-manifest","version":1,"key":"row/foreign","chunks":1,"total_bytes":1}"#, "manifest key does not match its lookup"),
    ];
    for (value, reason) in cases {
        raw(&store, "row/a", value)?;
        descriptor(roots(&store.snapshot()), "row/a", reason)?;
    }
    raw(&store, "row/a", br#"{"key":"row/a","payload":null}"#)?;
    check!(eq; roots(&store.snapshot())?.logical_roots(), 1);
    Ok(())
}

#[test]
fn malformed_logical_key_bytes_refuse_with_exact_categories() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut key = Store::journal_key(PHASE, "row/");
    key.push(255);
    store.journal.insert(&key, b"{}".as_slice())?;
    descriptor(
        roots(&store.snapshot()),
        "row/",
        "logical root key is not utf-8",
    )?;
    store.journal.remove(&key)?;
    raw(
        &store,
        "row/a\0b",
        br#"{"key":"row/a\u0000b","payload":null}"#,
    )?;
    text_refusal(roots(&store.snapshot()))?;
    Ok(())
}

#[test]
fn aliases_are_roots_without_claiming_backing_object_completeness() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    raw(&store, "row/alias", &ready_alias()?)?;
    check!(eq; roots(&store.snapshot())?.logical_roots(), 1);
    check!(
        matches!(store.snapshot().journal_object(PHASE, "row/alias"),
        Err(StoreError::JournalManifestMissing { phase, key, object })
            if phase == PHASE && key == "row/alias" && object == "a".repeat(64))
    );
    Ok(())
}

#[test]
fn unready_alias_refuses_with_its_actual_phase_key_and_state() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let alias = serde_json::to_vec(&json!({
        "type": "journal-alias", "version": 2, "state": "staging", "object": "a".repeat(64),
    }))?;
    raw(&store, "row/alias", &alias)?;
    check!(
        matches!(roots(&store.snapshot()), Err(StoreError::JournalNotReady { phase, key, state })
        if phase == PHASE && key == "row/alias" && state == "staging")
    );
    Ok(())
}

#[test]
fn alias_version_and_object_id_refusals_come_from_the_authoritative_codec() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let cases = [
        (
            json!({"type":"journal-alias","version":99,"state":"ready","object":"a".repeat(64)}),
            "alias descriptor record is not version 2",
        ),
        (
            json!({"type":"journal-alias","version":2,"state":"ready","object":"not-an-object"}),
            "alias descriptor has no 64-hex object id",
        ),
        (
            json!({"type":"journal-alias","version":2,"state":"ready","object":"A".repeat(64)}),
            "alias descriptor has no 64-hex object id",
        ),
    ];
    for (value, reason) in cases {
        raw(&store, "row/alias", &serde_json::to_vec(&value)?)?;
        descriptor(roots(&store.snapshot()), "row/alias", reason)?;
    }
    Ok(())
}

#[test]
fn replacing_chunk_bytes_changes_fingerprint_at_unchanged_membership_and_size() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    chunked(&store)?;
    let before = roots(&store.snapshot())?;
    let encoded = br#"{"key":"row/a","payload":{"revision":2}}"#;
    let key = chunk_key(&Store::journal_key(PHASE, "row/a"), 0)?;
    store
        .journal
        .insert(key, chunk_value("row/a", 0, 1, encoded)?)?;
    let after = roots(&store.snapshot())?;
    check!(eq; before.logical_roots(), after.logical_roots());
    check!(eq; before.fingerprint().physical_rows(), after.fingerprint().physical_rows());
    check!(eq; before.fingerprint().encoded_bytes(), after.fingerprint().encoded_bytes());
    check!(!before
        .fingerprint()
        .same_contents(&store, after.fingerprint())?);
    check!(eq; store.snapshot().journal_payload(PHASE, "row/a")?, Some(json!({"revision": 2})));
    Ok(())
}
