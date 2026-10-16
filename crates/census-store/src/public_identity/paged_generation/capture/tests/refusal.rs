use super::*;
mod diagnostic;

#[test]
fn disabled_input_revisions_refuse_without_implicit_enrollment() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    writer.enroll_manual_context_admission_v1(&store)?;
    let sequence = store.snapshot().sequence();
    assert!(writer
        .capture_raw_generation_v2(&store, &operation("disabled")?)
        .is_err());
    assert_eq!(store.snapshot().sequence(), sequence);
    assert_eq!(
        store.snapshot().public_identity_input_revisions_v2()?,
        crate::PublicIdentityInputRevisionsV2::Disabled
    );
    Ok(())
}

#[test]
fn oversize_physical_row_refuses_before_reservation_or_objects() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    seed(&store, 1)?;
    let (key, mut value) = raw(0)?;
    value.resize(MAX_RAW_VALUE_BYTES.checked_add(1).ok_or("row bound")?, b' ');
    store.entities.insert(key, value)?;
    let sequence = store.snapshot().sequence();
    let id = operation("row-budget")?;
    assert!(writer.capture_raw_generation_v2(&store, &id).is_err());
    assert_eq!(store.snapshot().sequence(), sequence);
    assert!(load_graph(&store, writer.store_identity(), &id)?.is_none());
    Ok(())
}

#[test]
fn aggregate_admission_refuses_before_reservation_or_objects() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    let mut batch = store.db.batch();
    for index in 0..97 {
        let (key, mut value) = raw(index)?;
        value.resize(MAX_RAW_VALUE_BYTES, b' ');
        batch.insert(&store.entities, key, value);
    }
    marks(&store, &mut batch, 97);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    let sequence = store.snapshot().sequence();
    let before_physical = diagnostic::physical_state(&store)?;
    eprintln!("aggregate before journal read: sequence={sequence}, state={before_physical:?}");
    let before = diagnostic::journal_state(&store)?;
    let admission: serde_json::Value = serde_json::from_slice(&before.2)?;
    assert_eq!(
        admission.get("roots").and_then(serde_json::Value::as_u64),
        Some(0)
    );
    assert_eq!(
        admission
            .get("reservations")
            .and_then(serde_json::Value::as_u64),
        Some(0)
    );
    let after_journal_read = diagnostic::physical_state(&store)?;
    eprintln!("aggregate after journal read: state={after_journal_read:?}, journal={before:?}");
    let measured = diagnostic::physical_bytes(&store)?;
    let after_measure = diagnostic::physical_state(&store)?;
    eprintln!(
        "aggregate after measurement: bytes={measured}, state={after_measure:?}, journal={:?}",
        diagnostic::journal_state(&store)?
    );
    assert_eq!(after_journal_read, before_physical);
    assert_eq!(after_measure, before_physical);
    assert_eq!(diagnostic::journal_state(&store)?, before);
    assert!(
        measured > 128 * 1024 * 1024,
        "aggregate fixture measured only {measured} bytes"
    );
    let id = operation("aggregate-budget")?;
    let result = writer.capture_raw_generation_v2(&store, &id);
    let after = diagnostic::journal_state(&store)?;
    eprintln!("aggregate capture result={result:?}; measured_physical_bytes={measured}; sequence={sequence}->{}; journal/header before={before:?}; after={after:?}", store.snapshot().sequence());
    match &result {
        Err(StoreError::PublicIdentity(crate::PublicIdentityError::AdmissionCapacity {
            resource: crate::PublicIdentityAdmissionResource::Bytes,
            needed,
            available,
        })) => {
            assert_eq!(*available, 128 * 1024 * 1024);
            assert!(*needed > *available);
            assert!(*needed >= measured);
        }
        other => return Err(format!("unexpected aggregate refusal: {other:?}").into()),
    }
    assert_eq!(diagnostic::physical_state(&store)?, before_physical);
    assert_eq!(after, before);
    assert!(load_graph(&store, writer.store_identity(), &id)?.is_none());
    Ok(())
}

fn first_page(store: &Store, retained: &RetainedRawGenerationV2) -> TestResult<ObjectRefV2> {
    let snapshot = store.snapshot();
    let seal = retained.verify(&snapshot)?;
    let binding = retained
        .binding()
        .table_binding(AuthorityTableV2::Athletes)?;
    let reference = seal.tables().first().ok_or("table")?;
    let table = snapshot.raw_generation_table(&reference.object(), &binding)?;
    let block = table.blocks().first().ok_or("block")?;
    let block = snapshot.raw_generation_block(&block.object(), &binding)?;
    Ok(block.pages().first().ok_or("page")?.object())
}

#[test]
fn missing_child_refuses_complete_replay_without_repair() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    seed(&store, 1)?;
    let id = operation("missing-child")?;
    let retained = writer
        .capture_raw_generation_v2(&store, &id)?
        .retained()
        .clone();
    let page = first_page(&store, &retained)?;
    store
        .journal
        .remove(crate::journal::object_manifest_key(page.root().as_str())?)?;
    let sequence = store.snapshot().sequence();
    assert!(retained.verify(&store.snapshot()).is_err());
    assert!(writer.capture_raw_generation_v2(&store, &id).is_err());
    assert_eq!(store.snapshot().sequence(), sequence);
    Ok(())
}

#[test]
fn tampered_child_refuses_complete_replay_without_repair() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    seed(&store, 1)?;
    let id = operation("tampered-child")?;
    let retained = writer
        .capture_raw_generation_v2(&store, &id)?
        .retained()
        .clone();
    let page = first_page(&store, &retained)?;
    let key = crate::journal::object_chunk_key(page.root().as_str(), 0)?;
    store.journal.insert(key, b"tampered retained chunk")?;
    let sequence = store.snapshot().sequence();
    assert!(retained.verify(&store.snapshot()).is_err());
    assert!(writer.capture_raw_generation_v2(&store, &id).is_err());
    assert_eq!(store.snapshot().sequence(), sequence);
    Ok(())
}
