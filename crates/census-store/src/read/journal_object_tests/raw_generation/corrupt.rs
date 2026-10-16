use super::*;

#[test]
fn retained_typed_decoder_rejects_duplicate_envelopes_and_nested_fields() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (binding, page) = page()?;
    let bytes = page.encode_jsonl()?;
    for (from, to) in [
        ("\"at\":\"\"", "\"at\":\"\",\"at\":\"\""),
        ("\"rows\":1", "\"rows\":1,\"rows\":1"),
        (
            "\"kind\":\"raw_page_header\"",
            "\"kind\":\"raw_page_header\",\"kind\":\"raw_page_header\"",
        ),
    ] {
        let malformed = changed(&bytes, from, to)?;
        let (_, object) = retain(&store, &malformed)?;
        match store.snapshot().raw_generation_page(&object, &binding) {
            Err(StoreError::Decode { source, .. }) => {
                check!(source.to_string().contains("duplicate field"))
            }
            other => return Err(format!("expected duplicate field, got {other:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn retained_typed_decoder_refuses_order_unknown_fields_and_extra_records() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (binding, page) = page()?;
    let bytes = page.encode_jsonl()?;
    let mut lines: Vec<_> = bytes.split_inclusive(|byte| *byte == b'\n').collect();
    lines.swap(0, 1);
    let reordered = lines.concat();
    let (_, object) = retain(&store, &reordered)?;
    match store.snapshot().raw_generation_page(&object, &binding) {
        Err(StoreError::Decode { source, .. }) => {
            check!(source.to_string().contains("unknown field"))
        }
        other => {
            return Err(format!("expected reordered header decode refusal, got {other:?}").into())
        }
    }
    let unknown = changed(&bytes, "\"rows\":1", "\"rows\":1,\"unknown\":0")?;
    let (_, object) = retain(&store, &unknown)?;
    match store.snapshot().raw_generation_page(&object, &binding) {
        Err(StoreError::Decode { source, .. }) => {
            check!(source.to_string().contains("unknown field"))
        }
        other => return Err(format!("expected unknown field, got {other:?}").into()),
    }
    let mut extra = bytes.clone();
    extra.extend_from_slice(
        bytes
            .split_inclusive(|byte| *byte == b'\n')
            .next()
            .ok_or("header")?,
    );
    let (_, object) = retain(&store, &extra)?;
    refused(
        store.snapshot().raw_generation_page(&object, &binding),
        "extra or missing JSONL records",
    )?;
    Ok(())
}

#[test]
fn retained_reader_refuses_corrupt_chunk_and_root_before_typed_decode() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (binding, page) = page()?;
    let (plan, object) = retain(&store, &page.encode_jsonl()?)?;
    let chunk = plan.chunks.first().ok_or("chunk")?;
    let mut changed_chunk = chunk.clone();
    *changed_chunk.first_mut().ok_or("byte")? = b'[';
    write(
        &store,
        &[(
            object_chunk_key(&plan.id, 0)?,
            serde_json::to_vec(&chunk_record(&plan.id, 0, &changed_chunk))?,
        )],
    )?;
    check!(matches!(
        store.snapshot().raw_generation_page(&object, &binding),
        Err(StoreError::JournalChunkDigest { .. })
    ));
    let forged = DigestV2::parse(&"f".repeat(64))?;
    write(&store, &pairs(&plan, forged.as_str())?)?;
    let reference = ObjectRefV2::new(
        forged,
        object.bytes(),
        object.records(),
        object.physical_bytes(),
    )?;
    check!(matches!(
        store.snapshot().raw_generation_page(&reference, &binding),
        Err(StoreError::JournalObjectDigest { .. })
    ));
    Ok(())
}

#[test]
fn retained_measurement_uses_full_pinned_physical_rows() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (binding, page) = page()?;
    let (plan, object) = retain(&store, &page.encode_jsonl()?)?;
    let pinned = store.snapshot();
    let wrong = ObjectRefV2::new(
        object.root().clone(),
        object.bytes(),
        object.records(),
        object.physical_bytes() + 1,
    )?;
    refused(
        pinned.raw_generation_page(&wrong, &binding),
        "retained physical bytes differ",
    )?;
    let small = ObjectRefV2::new(
        object.root().clone(),
        object.bytes(),
        object.records(),
        u64::from(object.bytes()),
    )?;
    check!(
        matches!(pinned.raw_generation_page(&small, &binding), Err(StoreError::JournalDescriptor { detail, .. }) if detail.contains("physical bytes exceed"))
    );
    let manifest_key = object_manifest_key(&plan.id)?;
    let mut raw = serde_json::to_vec(&manifest_record(&plan.id, &plan))?;
    raw.push(b' ');
    write(&store, &[(manifest_key, raw)])?;
    check!(eq; pinned.raw_generation_page(&object, &binding)?, page);
    check!(
        matches!(store.snapshot().raw_generation_page(&object, &binding), Err(StoreError::JournalDescriptor { detail, .. }) if detail.contains("physical bytes exceed"))
    );
    Ok(())
}

#[test]
fn retained_reader_refuses_extra_chunk_and_missing_chunk() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (binding, page) = page()?;
    let (plan, object) = retain(&store, &page.encode_jsonl()?)?;
    let index = plan.chunks.len();
    let key = object_chunk_key(&plan.id, index)?;
    write(
        &store,
        &[(
            key.clone(),
            serde_json::to_vec(&chunk_record(&plan.id, index, b""))?,
        )],
    )?;
    check!(matches!(
        store.snapshot().raw_generation_page(&object, &binding),
        Err(StoreError::JournalChunkExtra { .. })
    ));
    let mut batch = store.db.batch();
    batch.remove(&store.journal, &key);
    batch.remove(&store.journal, object_chunk_key(&plan.id, 0)?);
    batch.commit()?;
    check!(matches!(
        store.snapshot().raw_generation_page(&object, &binding),
        Err(StoreError::JournalChunkMissing { index: 0, .. })
    ));
    Ok(())
}

#[test]
fn retained_decoder_refuses_wrong_table_binding_and_manifest_record_count() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (binding, page) = page()?;
    let (_, object) = retain(&store, &page.encode_jsonl()?)?;
    let wrong_binding = capture()?.table_binding(AuthorityTableV2::Athletes)?;
    refused(
        store
            .snapshot()
            .raw_generation_page(&object, &wrong_binding),
        "raw page schema or table binding",
    )?;
    let wrong_count = ObjectRefV2::new(
        object.root().clone(),
        object.bytes(),
        object.records() + 1,
        object.physical_bytes(),
    )?;
    refused(
        store.snapshot().raw_generation_page(&wrong_count, &binding),
        "manifest differs from reference",
    )?;
    Ok(())
}

#[test]
fn retained_decoder_refuses_wrong_kind_and_noncanonical_field_order_exactly() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (binding, page) = page()?;
    let bytes = page.encode_jsonl()?;
    for (from, to, expected) in [
        (
            "\"kind\":\"raw_page_header\"",
            "\"kind\":\"wrong_kind\"",
            "typed record grammar",
        ),
        (
            "\"key\":\"header\",\"at\":\"\"",
            "\"at\":\"\",\"key\":\"header\"",
            "noncanonical typed JSONL bytes",
        ),
    ] {
        let malformed = changed(&bytes, from, to)?;
        let (_, object) = retain(&store, &malformed)?;
        match store.snapshot().raw_generation_page(&object, &binding) {
            Err(StoreError::Refused { detail }) => {
                check!(eq; detail, format!("raw public identity generation V2: {expected}"))
            }
            other => return Err(format!("expected exact {expected}, got {other:?}").into()),
        }
    }
    Ok(())
}
