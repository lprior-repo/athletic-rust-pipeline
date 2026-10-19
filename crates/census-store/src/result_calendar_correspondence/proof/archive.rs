use super::super::*;
use crate::RetainedCaptureRootV1;
use census_domain::evidence_subject_attachment::AttachmentObjectReferenceV1;
use census_domain::model::StoreIdentity;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

pub(super) const SCHEMA: &str = "result-calendar-source-closure-v2";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Archive<'a> {
    pub(super) schema: String,
    pub(super) store: StoreIdentity,
    pub(super) snapshot_sequence: u64,
    pub(super) inventory: ResultCalendarInventoryV1,
    #[serde(borrow)]
    pub(super) enrollment: &'a RawValue,
    #[serde(borrow)]
    pub(super) records: Vec<&'a RawValue>,
    #[serde(borrow)]
    pub(super) receipts: Vec<&'a RawValue>,
    #[serde(borrow)]
    pub(super) identity: &'a RawValue,
    pub(super) identity_digest: String,
    #[serde(borrow)]
    pub(super) attachments: &'a RawValue,
    pub(super) attachment_reference: AttachmentObjectReferenceV1,
    #[serde(borrow)]
    pub(super) external: Vec<Prefix<'a>>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Prefix<'a> {
    pub(super) root: RetainedCaptureRootV1,
    #[serde(borrow)]
    pub(super) rows: Vec<Row<'a>>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Row<'a> {
    pub(super) key: String,
    #[serde(borrow)]
    pub(super) value: &'a RawValue,
}

pub(super) fn raw(bytes: &[u8]) -> StoreResult<&RawValue> {
    let value: &RawValue =
        serde_json::from_slice(bytes).map_err(|_| Error::Corrupt("source closure JSON"))?;
    if value.get().as_bytes() != bytes {
        return Err(Error::Corrupt("source closure would rewrite physical bytes").into());
    }
    Ok(value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_envelope_outer_whitespace_refuses_instead_of_rewriting(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let original =
            b" \n{\"key\":\"original\",\"at\":\"retained\",\"payload\":{\"source\":1}}\r\n";
        check!(matches!(
            raw(original),
            Err(StoreError::ResultCalendarCorrespondence(_))
        ));
        Ok(())
    }

    #[test]
    fn physical_envelope_interior_whitespace_survives_serialization(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let original = br#"{"key":"original","at":"retained","payload": { "source": 1 }}"#;
        let retained = raw(original)?;
        check!(eq; serde_json::to_vec(retained)?, original);
        Ok(())
    }
}
