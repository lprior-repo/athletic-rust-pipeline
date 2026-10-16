use super::fixture::{reference, Context};
use crate::public_identity_tests::support::{PhysicalState, TestResult};
use crate::{Receipt, Store, MAX_JOURNAL_VALUE_BYTES};
use base64::{engine::general_purpose::STANDARD, Engine};
use census_domain::evidence_subject_attachment::{
    AttachmentObjectReferenceV1, ReviewedReferenceRecordV1,
};
use serde::{Deserialize, Serialize};

const RECORD_KEY: &str = "attachment/v1/record/00000000000000000001";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    at: String,
    key: String,
    payload: Entry,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    schema: String,
    record: ReviewedReferenceRecordV1,
    introduced: Vec<Inline>,
    inputs: Inputs,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Inline {
    storage: String,
    reference: AttachmentObjectReferenceV1,
    data: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Inputs {
    capture: AttachmentObjectReferenceV1,
    token: AttachmentObjectReferenceV1,
}

pub(super) fn exact_inline_append(
    before: &PhysicalState,
    after: &PhysicalState,
    context: &Context,
    issued: &[AttachmentObjectReferenceV1],
    record: &ReviewedReferenceRecordV1,
    receipt: &Receipt,
) -> TestResult {
    check!(eq; issued.len(), 4);
    let root = Store::journal_key("evidence_subject", RECORD_KEY);
    let journal = after
        .iter()
        .find(|(name, _)| name == crate::JOURNAL)
        .ok_or("journal absent")?;
    let raw = &journal
        .1
        .iter()
        .find(|(key, _)| key == &root)
        .ok_or("new inline root absent")?
        .1;
    check!(raw.len() <= MAX_JOURNAL_VALUE_BYTES);
    let actual: Envelope = serde_json::from_slice(raw)?;
    let introduced = exact_material(context, issued, record, &actual.payload.introduced)?;
    let expected = Envelope {
        at: record.committed_at.to_rfc3339(),
        key: RECORD_KEY.into(),
        payload: Entry {
            schema: "retained-evidence-subject-attachment-entry/v1".into(),
            record: record.clone(),
            introduced,
            inputs: Inputs {
                capture: issued.get(2).ok_or("issued capture absent")?.clone(),
                token: issued.get(3).ok_or("issued token absent")?.clone(),
            },
        },
    };
    let expected_root = census_domain::model::serialized_bytes(&expected)?;
    check!(eq; raw, &expected_root);
    check!(eq; actual.payload.record, *record);
    super::fixture::preserved_except_single_attachment(before, after, receipt)
}

fn exact_material(
    context: &Context,
    issued: &[AttachmentObjectReferenceV1],
    record: &ReviewedReferenceRecordV1,
    actual: &[Inline],
) -> TestResult<Vec<Inline>> {
    let references = record.command.reviewed_context.objects.as_slice();
    check!(eq; actual.len(), references.len());
    check!(eq; references.len(), context.objects.len().checked_add(issued.len()).ok_or("object count overflow")?);
    let mut expected = Vec::new();
    for (reference_in_command, retained) in references.iter().zip(actual) {
        check!(eq; retained.storage, "inline");
        check!(eq; &retained.reference, reference_in_command);
        let bytes = STANDARD.decode(&retained.data)?;
        check!(eq; reference(&bytes)?, *reference_in_command);
        let expected_bytes = match context
            .objects
            .iter()
            .find(|object| &object.reference == reference_in_command)
        {
            Some(object) => {
                check!(eq; bytes, object.bytes);
                object.bytes.as_slice()
            }
            None => {
                check!(issued.contains(reference_in_command));
                bytes.as_slice()
            }
        };
        expected.push(Inline {
            storage: "inline".into(),
            reference: reference_in_command.clone(),
            data: STANDARD.encode(expected_bytes),
        });
    }
    Ok(expected)
}
