use super::*;
use crate::read::JournalRawEnvelope;
use census_domain::evidence_subject_attachment::AttachmentListV1;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

pub(in super::super) const SCHEMA: &str = "retained-evidence-subject-attachment-entry/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in super::super) struct EntryInputs {
    pub(in super::super) capture: AttachmentObjectReferenceV1,
    pub(in super::super) token: AttachmentObjectReferenceV1,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Decoded<'a> {
    #[serde(borrow)]
    schema: &'a str,
    #[serde(borrow)]
    record: &'a RawValue,
    introduced: AttachmentListV1<Material>,
    inputs: EntryInputs,
}

#[derive(Serialize)]
pub(in super::super) struct Encoded<'a> {
    pub(in super::super) schema: &'static str,
    pub(in super::super) record: &'a ReviewedReferenceRecordV1,
    pub(in super::super) introduced: &'a [Material],
    pub(in super::super) inputs: &'a EntryInputs,
}

pub(super) fn decode(
    envelope: &JournalRawEnvelope,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<(CheckedReviewedReferenceRecordV1, Vec<Material>, EntryInputs)> {
    let raw = envelope.payload().get().as_bytes();
    let decoded = parse(raw, grant)?;
    let record = domain::decode_checked_reviewed_reference_with_grant(
        decoded.record.get().as_bytes(),
        ReplayLimits::SMALL.total_text_bytes,
        grant,
    )
    .map_err(Error::Domain)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    let at = chrono::DateTime::parse_from_rfc3339(envelope.at())
        .map_err(|_| Error::Corrupt("attachment envelope timestamp"))?;
    if at.with_timezone(&chrono::Utc) != record.committed_at {
        return Err(Error::Corrupt("attachment envelope and record time differ").into());
    }
    Ok((record, decoded.introduced.into_values(), decoded.inputs))
}

fn parse<'a>(raw: &'a [u8], grant: &mut AttachmentReplayGrantV1) -> StoreResult<Decoded<'a>> {
    grant.charge_bytes(raw.len()).map_err(Error::Domain)?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    if raw.first() != Some(&b'{') {
        return Err(Error::Corrupt("attachment entry must be an object").into());
    }
    let decoded: Decoded<'_> =
        serde_json::from_slice(raw).map_err(|source| Error::Domain(source.into()))?;
    if decoded.schema != SCHEMA {
        return Err(Error::Corrupt("attachment entry schema").into());
    }
    super::super::encoding::require_canonical(&decoded, raw, grant)?;
    Ok(decoded)
}

#[cfg(test)]
mod tests;
