use super::*;

#[test]
fn empty_and_control_containing_namespace_descriptors_refuse_precisely() -> TestResult {
    for (phase, prefix) in [
        ("", "row/"),
        (PHASE, ""),
        ("a\0b", "row/"),
        (PHASE, "row/\u{1}"),
        ("a\n", "row/"),
        (PHASE, "row/\u{85}"),
    ] {
        text_refusal(JournalNamespace::new(phase, prefix))?;
    }
    Ok(())
}

#[test]
fn reserved_object_phase_has_its_own_refusal() -> TestResult {
    check!(
        matches!(JournalNamespace::new("j2", "manifest/"), Err(StoreError::Refused { detail })
        if detail == "shared object storage is not a logical journal namespace")
    );
    let adjacent = JournalNamespace::new("j20", "manifest/")?;
    check!(eq; adjacent.phase(), "j20");
    check!(eq; adjacent.logical_prefix(), "manifest/");
    Ok(())
}

#[test]
fn selector_length_limit_counts_the_phase_separator_and_utf8_bytes() -> TestResult {
    let phase = "é";
    let prefix_len = crate::MAX_JOURNAL_KEY_BYTES
        .checked_sub(phase.len())
        .and_then(|size| size.checked_sub(1))
        .ok_or(StoreError::CounterOverflow)?;
    let prefix = "x".repeat(prefix_len);
    let exact = JournalNamespace::new(phase, &prefix)?;
    check!(eq; exact.physical_prefix()?.len(), crate::MAX_JOURNAL_KEY_BYTES);
    let longer = format!("{prefix}x");
    let needed = crate::MAX_JOURNAL_KEY_BYTES
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    check!(
        matches!(JournalNamespace::new(phase, &longer), Err(StoreError::RowTooLarge { bytes, max })
        if bytes == needed && max == crate::MAX_JOURNAL_KEY_BYTES)
    );
    Ok(())
}

#[test]
fn exact_maximum_root_key_is_accepted_and_one_more_refuses_inventory() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let logical_length = crate::MAX_JOURNAL_KEY_BYTES
        .checked_sub(PHASE.len())
        .and_then(|size| size.checked_sub(1))
        .ok_or(StoreError::CounterOverflow)?;
    let tail_length = logical_length
        .checked_sub(4)
        .ok_or(StoreError::CounterOverflow)?;
    let exact = format!("row/{}", "x".repeat(tail_length));
    let value = serde_json::to_vec(&json!({"key": exact, "payload": null}))?;
    raw(&store, &exact, &value)?;
    let inventory = roots(&store.snapshot())?;
    check!(eq; inventory.logical_roots(), 1);
    check!(eq; inventory.fingerprint().physical_rows(), 1);
    store.journal.remove(Store::journal_key(PHASE, &exact))?;
    let longer = format!("{exact}x");
    raw(&store, &longer, b"{")?;
    let raw_scan = store
        .snapshot()
        .journal_namespace_fingerprint(namespace()?, budget(1))?;
    check!(eq; raw_scan.physical_rows(), 1);
    let needed = crate::MAX_JOURNAL_KEY_BYTES
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    check!(
        matches!(roots(&store.snapshot()), Err(StoreError::RowTooLarge { bytes, max })
        if bytes == needed && max == crate::MAX_JOURNAL_KEY_BYTES)
    );
    Ok(())
}

#[test]
fn maximum_root_with_genuine_five_byte_chunk_suffix_is_accepted() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let tail_length = crate::MAX_JOURNAL_KEY_BYTES
        .checked_sub(PHASE.len())
        .and_then(|size| size.checked_sub(5))
        .ok_or(StoreError::CounterOverflow)?;
    let logical = format!("row/{}", "x".repeat(tail_length));
    let physical = Store::journal_key(PHASE, &logical);
    check!(eq; physical.len(), crate::MAX_JOURNAL_KEY_BYTES);
    raw(&store, &logical, &manifest_value(&logical, 1, 4)?)?;
    let chunk = chunk_key(&physical, 0)?;
    let needed = crate::MAX_JOURNAL_KEY_BYTES
        .checked_add(5)
        .ok_or(StoreError::CounterOverflow)?;
    check!(eq; chunk.len(), needed);
    store.journal.insert(chunk, b"opaque chunk".as_slice())?;
    let inventory = roots(&store.snapshot())?;
    check!(eq; inventory.logical_roots(), 1);
    check!(eq; inventory.fingerprint().physical_rows(), 2);
    Ok(())
}

#[test]
fn non_ascii_phase_and_prefix_keep_exact_adjacent_boundaries() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    put(&store, "étape", "行/a", 1)?;
    put(&store, "étape", "行列/a", 2)?;
    put(&store, "étapes", "行/a", 3)?;
    let view = store.snapshot();
    let mut seen = Vec::new();
    let inventory = view.for_each_journal_root_bounded(
        JournalNamespace::new("étape", "行/")?,
        budget(1),
        |key| {
            seen.push(key.to_string());
            Ok(())
        },
    )?;
    check!(eq; seen, vec!["行/a".to_string()]);
    check!(eq; inventory.logical_roots(), 1);
    let broad = view.for_each_journal_root_bounded(
        JournalNamespace::new("étape", "行")?,
        budget(2),
        |_| Ok(()),
    )?;
    check!(eq; broad.logical_roots(), 2);
    check!(!inventory
        .fingerprint()
        .same_contents(&store, broad.fingerprint())?);
    Ok(())
}

#[test]
fn prefix_without_slash_intentionally_includes_adjacent_text() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    put(&store, PHASE, "row/a", 1)?;
    put(&store, PHASE, "rows/a", 2)?;
    put(&store, PHASE, "rowboat", 3)?;
    let view = store.snapshot();
    let broad = view.for_each_journal_root_bounded(
        JournalNamespace::new(PHASE, "row")?,
        budget(3),
        |_| Ok(()),
    )?;
    check!(eq; broad.logical_roots(), 3);
    check!(eq; roots(&view)?.logical_roots(), 1);
    Ok(())
}
