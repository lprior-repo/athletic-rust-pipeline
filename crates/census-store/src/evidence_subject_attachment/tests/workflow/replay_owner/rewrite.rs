use super::*;
use crate::evidence_subject_attachment::{
    ledger::wire::{Encoded, EntryInputs, SCHEMA},
    material::Material,
};
use serde::Serialize;

#[derive(Serialize)]
struct Envelope<'a> {
    at: String,
    key: &'a str,
    payload: Encoded<'a>,
}

pub(super) fn replace_record_time(case: &Committed) -> TestResult<ReviewedReferenceRecordV1> {
    let key = Store::journal_key("evidence_subject", RECORD_KEY);
    let raw = case
        .fixture
        .store
        .journal
        .get(&key)?
        .ok_or("original attachment root absent")?;
    let original: serde_json::Value = serde_json::from_slice(&raw)?;
    let payload = original.get("payload").ok_or("original payload absent")?;
    let introduced: Vec<Material> = serde_json::from_value(
        payload
            .get("introduced")
            .ok_or("original material absent")?
            .clone(),
    )?;
    let inputs: EntryInputs = serde_json::from_value(
        payload
            .get("inputs")
            .ok_or("original input roles absent")?
            .clone(),
    )?;
    let mut changed = case.record.clone();
    changed.committed_at = changed
        .committed_at
        .checked_add_signed(chrono::Duration::seconds(1))
        .ok_or("controlled timestamp overflow")?;
    changed.record_digest = changed.calculated_record_digest()?;
    let envelope = Envelope {
        at: changed.committed_at.to_rfc3339(),
        key: RECORD_KEY,
        payload: Encoded {
            schema: SCHEMA,
            record: &changed,
            introduced: &introduced,
            inputs: &inputs,
        },
    };
    let bytes = serde_json::to_vec(&envelope)?;
    check!(bytes.len() <= crate::MAX_JOURNAL_VALUE_BYTES);
    let candidate: serde_json::Value = serde_json::from_slice(&bytes)?;
    check!(eq; candidate.pointer("/payload/introduced"), original.pointer("/payload/introduced"));
    check!(eq; candidate.pointer("/payload/inputs"), original.pointer("/payload/inputs"));
    check!(eq; candidate.pointer("/payload/record/command"), original.pointer("/payload/record/command"));
    let mut receipt = case.receipt.clone();
    receipt.digest = changed.record_digest.as_str().to_owned();
    receipt.at = changed.committed_at.date_naive().to_string();
    let receipt_bytes = serde_json::to_vec(&receipt)?;
    case.fixture.store.journal.insert(key, bytes)?;
    case.fixture
        .store
        .receipts
        .insert(receipt.operation.as_bytes(), receipt_bytes)?;
    Ok(changed)
}
