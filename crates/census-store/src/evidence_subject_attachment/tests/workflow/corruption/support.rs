use super::*;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

pub(super) const RECORD_KEY: &str = "attachment/v1/record/00000000000000000001";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CorruptedEnvelope {
    pub(super) at: String,
    pub(super) key: String,
    pub(super) payload: CorruptedEntry,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CorruptedEntry {
    pub(super) schema: String,
    pub(super) record: Box<RawValue>,
    pub(super) introduced: Vec<InlineMaterial>,
    pub(super) inputs: CorruptedInputs,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InlineMaterial {
    pub(super) storage: String,
    pub(super) reference: AttachmentObjectReferenceV1,
    pub(super) data: String,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CorruptedInputs {
    pub(super) capture: AttachmentObjectReferenceV1,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) token: Option<AttachmentObjectReferenceV1>,
}

pub(super) fn committed(label: &str) -> TestResult<(Fixture, Context, ReviewedReferenceRecordV1)> {
    let fixture = Fixture::new()?;
    let context = Context::synthetic(label)?;
    let inspection = fixture.inspect()?;
    let command = context.command(label, &inspection)?;
    written(apply(
        &fixture.store,
        prepare(&fixture.store, inspection, command, &context)?,
    )?)?;
    let record = first_record(&fixture.store)?;
    Ok((fixture, context, record))
}

pub(super) fn entry(store: &Store) -> TestResult<Value> {
    let raw = store
        .journal
        .get(Store::journal_key("evidence_subject", RECORD_KEY))?
        .ok_or("first inline entry absent")?;
    Ok(serde_json::from_slice(&raw)?)
}

pub(super) fn replace_entry_preserving_record(
    store: &Store,
    altered: &Value,
    original: &ReviewedReferenceRecordV1,
) -> TestResult {
    check!(eq; altered.pointer("/payload/record"), Some(&serde_json::to_value(original)?));
    let original_raw = store
        .journal
        .get(Store::journal_key("evidence_subject", RECORD_KEY))?
        .ok_or("original inline entry absent")?;
    let mut envelope: CorruptedEnvelope = serde_json::from_slice(&original_raw)?;
    let original_record_raw = envelope.payload.record.get().to_owned();
    envelope.payload.inputs = serde_json::from_value(
        altered
            .pointer("/payload/inputs")
            .ok_or("altered input roles absent")?
            .clone(),
    )?;
    let raw = serde_json::to_vec(&envelope)?;
    check!(eq; serde_json::from_slice::<Value>(&raw)?, *altered);
    replace_typed_entry(store, RECORD_KEY, &envelope, &original_record_raw)
}

pub(super) fn replace_typed_entry(
    store: &Store,
    key: &str,
    envelope: &CorruptedEnvelope,
    original_record_raw: &str,
) -> TestResult {
    let raw = serde_json::to_vec(envelope)?;
    let retained: CorruptedEnvelope = serde_json::from_slice(&raw)?;
    check!(eq; retained.payload.record.get(), original_record_raw);
    check!(raw.len() <= crate::MAX_JOURNAL_VALUE_BYTES);
    store
        .journal
        .insert(Store::journal_key("evidence_subject", key), raw)?;
    Ok(())
}

pub(super) fn poisoned_without_further_reads(
    store: &Store,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult {
    let usage = grant.usage();
    check!(matches!(
        store
            .snapshot()
            .read_evidence_subject_attachment_references_v1(grant),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Domain(EvidenceSubjectAttachmentError::Poisoned)
        ))
    ));
    check!(eq; grant.usage(), usage);
    Ok(())
}
