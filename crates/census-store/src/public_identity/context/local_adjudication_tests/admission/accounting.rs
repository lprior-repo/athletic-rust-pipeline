use super::*;
use fjall::Readable;
use serde_json::Value;

const HEADER: &str = "manual-context-admission-v1\0header";

#[test]
fn graph_preparation_charges_only_exact_metadata_once() -> TestResult {
    let fixture = merged()?;
    let request = request(&fixture)?;
    let operation = "admission-exact-metadata";
    stage_inventory(&fixture, operation)?;
    let before = admission_counts(&fixture.store)?;
    let committed = committed_state(&fixture.store)?;
    let head = fixture.writer.head(&fixture.store)?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        operation,
        request.clone(),
        fixture.store.snapshot().revision(),
        &head,
    )?;
    let key = format!(
        "manual-context-admission-v1\0reservation\0{}",
        crate::journal::object_digest(operation.as_bytes())
    );
    let raw = fixture
        .store
        .db
        .snapshot()
        .get(&fixture.store.journal, &key)?
        .ok_or("preparation")?;
    let held = crate::public_identity::local_adjudication::admission::preparation::load(
        &fixture.store,
        fixture.writer.store_identity(),
        operation,
    )?
    .ok_or("preparation")?;
    check!(eq; held.version, 4);
    check!(eq; &held.graph, prepared.graph());
    let metadata = u64::try_from(
        key.len()
            .checked_add(raw.len())
            .ok_or("metadata overflow")?,
    )?;
    check!(eq; admission_counts(&fixture.store)?, (before.0.checked_add(metadata).ok_or("bytes")?, before.1, before.2.checked_add(1).ok_or("reservations")?));
    check!(eq; committed_state(&fixture.store)?, committed);
    let state = retained_state(&fixture.store)?;
    let retry = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        operation,
        request,
        fixture.store.snapshot().revision(),
        &head,
    )?;
    check!(eq; retry.approval_bytes()?, prepared.approval_bytes()?);
    check!(eq; retained_state(&fixture.store)?, state);
    Ok(())
}

#[test]
fn undercharged_retry_refuses_before_any_graph_or_history_write() -> TestResult {
    refuse_corrupt_header("bytes", 0)
}

#[test]
fn forged_shared_budget_counters_refuse_new_and_existing_operations() -> TestResult {
    for (field, bound) in [
        ("bytes", 128 * 1024 * 1024),
        ("roots", 4096),
        ("reservations", 4096),
    ] {
        refuse_corrupt_header(field, bound)?;
        refuse_corrupt_header(field, bound.checked_add(1).ok_or("bound")?)?;
    }
    Ok(())
}

fn refuse_corrupt_header(field: &str, value: u64) -> TestResult {
    let fixture = merged()?;
    let request = request(&fixture)?;
    let operation = "admission-corrupt-budget";
    let head = fixture.writer.head(&fixture.store)?;
    fixture.writer.prepare_local_adjudication(
        &fixture.store,
        operation,
        request.clone(),
        fixture.store.snapshot().revision(),
        &head,
    )?;
    let original = fixture.store.journal.get(HEADER)?.ok_or("header")?;
    let mut header: Value = serde_json::from_slice(&original)?;
    *header.get_mut(field).ok_or("header counter")? = value.into();
    let mut batch = fixture.store.db.batch();
    batch.insert(&fixture.store.journal, HEADER, serde_json::to_vec(&header)?);
    batch
        .durability(Some(fjall::PersistMode::SyncData))
        .commit()?;
    let before = retained_state(&fixture.store)?;
    for operation in [operation, "admission-corrupt-budget-new"] {
        let result = fixture.writer.prepare_local_adjudication(
            &fixture.store,
            operation,
            request.clone(),
            fixture.store.snapshot().revision(),
            &head,
        );
        check!(matches!(
            result,
            Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
                "manual admission missing, corrupt, or exhausted"
            )))
        ));
        check!(eq; retained_state(&fixture.store)?, before);
    }
    let mut batch = fixture.store.db.batch();
    batch.insert(&fixture.store.journal, HEADER, original.as_ref());
    batch
        .durability(Some(fjall::PersistMode::SyncData))
        .commit()?;
    let before = retained_state(&fixture.store)?;
    fixture.writer.prepare_local_adjudication(
        &fixture.store,
        operation,
        request,
        fixture.store.snapshot().revision(),
        &head,
    )?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}
