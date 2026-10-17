use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;

#[test]
fn replay_owner_revalidates_changed_same_length_material_before_returning_a_receipt() -> TestResult
{
    let case = committed("replay-owner-material")?;
    let prepared = case.fixture.store.prepare_evidence_subject_attachment_v1(
        case.record.command.clone(),
        Preparation::ReplayOnly,
    )?;
    let object = case
        .context
        .objects
        .first()
        .ok_or("inline material absent")?;
    let key = Store::journal_key("evidence_subject", RECORD_KEY);
    let raw = case
        .fixture
        .store
        .journal
        .get(&key)?
        .ok_or("attachment root absent")?;
    let changed = replace_ascii(&raw, STANDARD.encode(&object.bytes).as_bytes())?;
    let original: Value = serde_json::from_slice(&raw)?;
    let altered: Value = serde_json::from_slice(&changed)?;
    check!(eq; original.pointer("/payload/record"), altered.pointer("/payload/record"));
    check!(eq; original.pointer("/payload/inputs"), altered.pointer("/payload/inputs"));
    case.fixture.store.journal.insert(key, changed)?;
    refuse_corrupt(&case.fixture.store, prepared, "inline object digest")
}

#[test]
fn replay_owner_revalidates_receipt_only_change_against_unchanged_history() -> TestResult {
    let case = committed("replay-owner-receipt")?;
    let prepared = case.fixture.store.prepare_evidence_subject_attachment_v1(
        case.record.command.clone(),
        Preparation::ReplayOnly,
    )?;
    let raw = case
        .fixture
        .store
        .receipts
        .get(case.receipt.operation.as_bytes())?
        .ok_or("receipt absent")?;
    let changed = replace_ascii(&raw, case.receipt.digest.as_bytes())?;
    let original: Value = serde_json::from_slice(&raw)?;
    let altered: Value = serde_json::from_slice(&changed)?;
    check!(eq; original.get("operation"), altered.get("operation"));
    check!(ne; original.get("digest"), altered.get("digest"));
    case.fixture
        .store
        .receipts
        .insert(case.receipt.operation.as_bytes(), changed)?;
    refuse_corrupt(
        &case.fixture.store,
        prepared,
        "attachment record and receipt differ",
    )
}

#[test]
fn replay_owner_missing_durable_record_cannot_recreate_the_original_append() -> TestResult {
    let case = committed("replay-owner-disappeared")?;
    let prepared = case.fixture.store.prepare_evidence_subject_attachment_v1(
        case.record.command.clone(),
        Preparation::ReplayOnly,
    )?;
    case.fixture
        .store
        .journal
        .remove(Store::journal_key("evidence_subject", RECORD_KEY))?;
    case.fixture
        .store
        .receipts
        .remove(case.receipt.operation.as_bytes())?;
    refuse_corrupt(
        &case.fixture.store,
        prepared,
        "replay-only prepared record disappeared",
    )
}

#[test]
fn replay_owner_changed_canonical_record_refuses_even_when_command_and_approval_are_identical(
) -> TestResult {
    let case = committed("replay-owner-full-record")?;
    let prepared = case.fixture.store.prepare_evidence_subject_attachment_v1(
        case.record.command.clone(),
        Preparation::ReplayOnly,
    )?;
    let changed = super::rewrite::replace_record_time(&case)?;
    check!(eq; changed.command, case.record.command);
    check!(eq; changed.approval_digest, case.record.approval_digest);
    check!(ne; changed.record_digest, case.record.record_digest);
    check!(eq; first_record(&case.fixture.store)?, changed);
    let before = retained_state(&case.fixture.store)?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(matches!(
        case.fixture
            .store
            .apply_evidence_subject_attachment_v1(prepared, &approval, &digest),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::OperationConflict
        ))
    ));
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}
