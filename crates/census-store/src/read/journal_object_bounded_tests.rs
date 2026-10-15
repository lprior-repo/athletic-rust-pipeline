use super::*;

#[test]
fn object_cursors_retain_only_one_page_and_chunk() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let count = OBJECT_INDEX_PAGE_ENTRIES * 3 + 7;
    let records: Vec<_> = (0..count)
        .map(|index| record(&index.to_string(), json!({"blob": "x".repeat(2048)})))
        .collect();
    let staged = stage_records(&store, &records)?;
    check!(staged.chunks.len() > 3);
    let object = read(&store)?.ok_or("object")?;
    let mut cursor = object.records();
    check!(eq; cursor.retained_capacity(), (0, 0));
    let mut independent = object.records();
    for expected in 0..count {
        let row = cursor.next().ok_or("record")??;
        check!(eq; row.sequence(), expected);
        check!(eq; row.entry().key, expected.to_string());
        check!(eq; row.offset(), staged.ranges[expected].0);
        check!(eq; row.length(), staged.ranges[expected].1);
        let (ranges, chunk) = cursor.retained_capacity();
        check!(ranges <= OBJECT_INDEX_PAGE_ENTRIES);
        check!(chunk <= OBJECT_CHUNK_BYTES + 2);
    }
    check!(cursor.next().is_none());
    check!(eq; independent.next().ok_or("independent")??.sequence(), 0);
    check!(eq; object.records().next().ok_or("restart")??.sequence(), 0);
    Ok(())
}

#[test]
fn borrowed_object_releases_before_store_reopen() -> TestResult {
    let dir = tempfile::tempdir()?;
    {
        let store = Store::open(dir.path())?;
        stage_records(&store, &entries(&[("original", 1)]))?;
        let object = store
            .snapshot()
            .journal_object(PHASE, KEY)?
            .ok_or("object")?;
        check!(eq; object.records().next().ok_or("record")??.entry().key, "original");
    }
    let reopened = Store::open(dir.path())?;
    let object = read(&reopened)?.ok_or("reopened object")?;
    check!(eq; object.records().next().ok_or("record")??.entry().key, "original");
    Ok(())
}

#[test]
fn pinned_object_reads_old_pages_and_chunks_after_physical_overwrite() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let staged = stage_records(&store, &[large("first"), large("second")])?;
    let object = read(&store)?.ok_or("object")?;
    let mut cursor = object.records();
    check!(eq; cursor.next().ok_or("first")??.entry().key, "first");
    let replacements = pairs(&staged, &staged.id)?
        .into_iter()
        .map(|(key, _)| (key, b"invalid replacement".to_vec()))
        .collect::<Vec<_>>();
    write(&store, &replacements)?;
    check!(eq; cursor.next().ok_or("second")??.entry().key, "second");
    check!(cursor.next().is_none());
    check!(eq; stream(&object)?.len(), 2);
    check!(read(&store).is_err());
    Ok(())
}

#[test]
fn object_preflights_each_physical_row_before_decoding() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let staged = plan(&entries(&[("first", 1)]))?;
    let keys = [
        Store::journal_key(PHASE, KEY),
        object_manifest_key(&staged.id)?,
        object_index_key(&staged.id, 0)?,
        object_chunk_key(&staged.id, 0)?,
    ];
    for key in keys {
        stage(&store, &staged)?;
        write(&store, &[(key, vec![b'!'; MAX_JOURNAL_VALUE_BYTES + 1])])?;
        check!(matches!(read(&store), Err(StoreError::JournalReadLimit {
            what: "physical object row", needed, max, ..
        }) if needed == MAX_JOURNAL_VALUE_BYTES + 1 && max == MAX_JOURNAL_VALUE_BYTES));
    }
    Ok(())
}

#[test]
fn object_validates_late_chunks_before_returning_any_records() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let staged = stage_records(&store, &[large("first"), large("second")])?;
    let last = staged.chunks.len() - 1;
    write(
        &store,
        &[(object_chunk_key(&staged.id, last)?, b"invalid".to_vec())],
    )?;
    check!(matches!(read(&store), Err(StoreError::Decode { .. })));
    Ok(())
}

#[test]
fn object_preserves_unknown_envelope_fields() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let staged = plan(&entries(&[("first", 1)]))?;
    let mut rows = pairs(&staged, &staged.id)?;
    rows.push(alias_pair(&staged.id)?);
    for (_, bytes) in &mut rows {
        let mut value: Value = serde_json::from_slice(bytes)?;
        value["historical_extension"] = json!({"ignored": true});
        *bytes = serde_json::to_vec(&value)?;
    }
    write(&store, &rows)?;
    let object = read(&store)?.ok_or("object")?;
    check!(eq; stream(&object)?.len(), 1);
    Ok(())
}

#[test]
fn object_preserves_index_framing_without_newline_delimiters() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut payload = Vec::new();
    let mut ranges = Vec::new();
    for entry in entries(&[("first", 1), ("second", 2)]) {
        let bytes = serde_json::to_vec(&entry)?;
        ranges.push((payload.len(), bytes.len()));
        payload.extend_from_slice(&bytes);
    }
    let staged = ObjectPlan {
        id: object_digest(&payload),
        chunks: vec![payload.clone()],
        payload,
        ranges,
    };
    stage(&store, &staged)?;
    let object = read(&store)?.ok_or("object")?;
    let rows = stream(&object)?;
    check!(eq; rows.len(), 2);
    check!(eq; rows[0].entry().key.as_str(), "first");
    check!(eq; rows[1].entry().key.as_str(), "second");
    check!(eq; rows[1].offset(), staged.ranges[1].0);
    Ok(())
}
