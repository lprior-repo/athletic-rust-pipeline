use super::support::*;
use crate::public_identity_tests::support::TestResult;
use crate::Store;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;

const RECORD_KEY: &str = "attachment/v1/record/00000000000000000001";

fn material_case(boundary: Boundary) -> TestResult {
    let case = committed("owner-reuse-original-material")?;
    let object = case
        .context
        .objects
        .first()
        .ok_or("original inline material absent")?;
    let encoded = STANDARD.encode(&object.bytes);
    refuses_at_boundary(
        &case.fixture,
        boundary,
        |store| {
            let key = Store::journal_key("evidence_subject", RECORD_KEY);
            let raw = store
                .journal
                .get(&key)?
                .ok_or("original journal root absent")?;
            let changed = replace_one_ascii(&raw, encoded.as_bytes())?;
            let original: Value = serde_json::from_slice(&raw)?;
            let altered: Value = serde_json::from_slice(&changed)?;
            check!(eq; original.pointer("/payload/record"), altered.pointer("/payload/record"));
            check!(eq; original.pointer("/payload/inputs"), altered.pointer("/payload/inputs"));
            check!(eq; original.pointer("/payload/record"), Some(&serde_json::to_value(&case.record)?));
            let evidence = store.evidence_generation();
            let derived = store.derived_generation();
            store.journal.insert(key, changed)?;
            check!(eq; store.evidence_generation(), evidence);
            check!(eq; store.derived_generation(), derived);
            Ok(())
        },
        "inline object digest",
    )
}

#[test]
fn raw_same_length_prior_material_change_after_inspection_is_not_hidden_by_owner_reuse(
) -> TestResult {
    material_case(Boundary::Preparation)
}

#[test]
fn raw_same_length_prior_material_change_after_preparation_is_not_hidden_by_owner_reuse(
) -> TestResult {
    material_case(Boundary::Application)
}

fn receipt_case(boundary: Boundary) -> TestResult {
    let case = committed("owner-reuse-original-receipt")?;
    refuses_at_boundary(
        &case.fixture,
        boundary,
        |store| {
            let key = case.receipt.operation.as_bytes();
            let raw = store.receipts.get(key)?.ok_or("original receipt absent")?;
            let changed = replace_one_ascii(&raw, case.receipt.digest.as_bytes())?;
            let original: Value = serde_json::from_slice(&raw)?;
            let altered: Value = serde_json::from_slice(&changed)?;
            check!(eq; original.get("operation"), altered.get("operation"));
            check!(ne; original.get("digest"), altered.get("digest"));
            let evidence = store.evidence_generation();
            let derived = store.derived_generation();
            store.receipts.insert(key, changed)?;
            check!(eq; store.evidence_generation(), evidence);
            check!(eq; store.derived_generation(), derived);
            Ok(())
        },
        "attachment record and receipt differ",
    )
}

#[test]
fn raw_same_length_receipt_change_after_inspection_is_checked_against_held_history() -> TestResult {
    receipt_case(Boundary::Preparation)
}

#[test]
fn raw_same_length_receipt_change_after_preparation_is_checked_against_held_history() -> TestResult
{
    receipt_case(Boundary::Application)
}
