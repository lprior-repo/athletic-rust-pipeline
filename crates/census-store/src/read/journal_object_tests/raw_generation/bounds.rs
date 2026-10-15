use super::*;

#[test]
fn manifest_reference_limits_refuse_before_missing_index_or_chunk() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (binding, page) = page()?;
    let plan = raw_plan(&page.encode_jsonl()?)?;
    write(
        &store,
        &[(
            object_manifest_key(&plan.id)?,
            serde_json::to_vec(&manifest_record(&plan.id, &plan))?,
        )],
    )?;
    let bytes = u32::try_from(plan.payload.len())?;
    let count = u32::try_from(plan.ranges.len())?;
    for (bytes, records) in [(bytes - 1, count), (bytes, count - 1)] {
        let object =
            ObjectRefV2::new(DigestV2::parse(&plan.id)?, bytes, records, u64::from(bytes))?;
        check!(
            matches!(store.snapshot().raw_generation_page(&object, &binding), Err(StoreError::JournalDescriptor { detail, .. }) if detail == "authority context exceeds retained input limits")
        );
    }
    Ok(())
}

#[test]
fn oversized_record_refuses_before_missing_chunk_and_typed_allocation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (binding, _) = page()?;
    let mut bytes = vec![b' '; MAX_RECORD_BYTES];
    bytes.push(b'\n');
    let plan = raw_plan(&bytes)?;
    let mut rows = pairs(&plan, &plan.id)?;
    let prefix = crate::journal::object_chunk_prefix(&plan.id)?;
    rows.retain(|(key, _)| !key.starts_with(&prefix));
    write(&store, &rows)?;
    let object = ObjectRefV2::new(
        DigestV2::parse(&plan.id)?,
        u32::try_from(bytes.len())?,
        1,
        u64::try_from(bytes.len())?,
    )?;
    check!(
        matches!(store.snapshot().raw_generation_page(&object, &binding), Err(StoreError::RowTooLarge { bytes, max }) if bytes == MAX_RECORD_BYTES + 1 && max == MAX_RECORD_BYTES)
    );
    Ok(())
}

#[test]
fn exact_record_ceiling_reaches_typed_parser_and_kind_ceiling_precedes_reads() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (binding, _) = page()?;
    let mut bytes = vec![b' '; MAX_RECORD_BYTES - 1];
    bytes.push(b'\n');
    let (_, object) = retain(&store, &bytes)?;
    check!(matches!(
        store.snapshot().raw_generation_page(&object, &binding),
        Err(StoreError::Decode { .. })
    ));
    let object = ObjectRefV2::new(
        DigestV2::parse(&"e".repeat(64))?,
        MAX_DIRECTORY_JSONL_BYTES + 1,
        1,
        u64::from(MAX_DIRECTORY_JSONL_BYTES + 1),
    )?;
    refused(
        store.snapshot().raw_generation_table(&object, &binding),
        "exceeds kind limits",
    )?;
    Ok(())
}

#[test]
fn index_sequence_and_range_mutations_refuse_before_chunk_reads() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (binding, page) = page()?;
    let (plan, object) = retain(&store, &page.encode_jsonl()?)?;
    for (field, value, reason) in [
        ("sequence", json!(1), "sequence does not match"),
        ("offset", json!(1), "ranges do not tile"),
    ] {
        let mut index = index_record(&plan.id, 0, &plan.ranges, 0);
        let entry = index
            .get_mut("entries")
            .and_then(Value::as_array_mut)
            .and_then(|entries| entries.first_mut())
            .ok_or("entry")?;
        *entry.get_mut(field).ok_or("field")? = value;
        write(
            &store,
            &[(object_index_key(&plan.id, 0)?, serde_json::to_vec(&index)?)],
        )?;
        check!(
            matches!(store.snapshot().raw_generation_page(&object, &binding), Err(StoreError::JournalDescriptor { detail, .. }) if detail.contains(reason))
        );
    }
    Ok(())
}
