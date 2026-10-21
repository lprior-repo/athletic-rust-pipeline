use super::*;
use census_store::{RetainedCaptureRootV1, StoreSnapshot};
use serde_json::value::RawValue;
use std::collections::BTreeMap;

pub(super) fn capture(
    snapshot: &StoreSnapshot<'_>,
    objects: &mut Objects,
    root: &RetainedCaptureRootV1,
    field: &str,
    slot: RetainedCaptureSlotV1,
    original: &[u8],
) -> TestResult<(AttachmentDigestV1, AttachmentDigestV1)> {
    let raw = snapshot
        .journal_payload_raw_bounded(
            root.schema.phase(),
            &root.key,
            JournalReadBudget::new(4 * 1024 * 1024)?,
        )?
        .ok_or("actual retained context packet")?;
    check!(eq; objects::reference(raw.get().as_bytes())?,root.payload);
    let fields: BTreeMap<String, Box<RawValue>> = serde_json::from_str(raw.get())?;
    let capture = fields.get(field).ok_or("literal context capture slot")?;
    let json = objects.retained(RetainedCaptureLeafV1 {
        version: 1,
        root: root.clone(),
        slot,
        kind: RetainedCaptureLeafKindV1::CaptureJson,
        object: objects::reference(capture.get().as_bytes())?,
    })?;
    let body = objects.retained(RetainedCaptureLeafV1 {
        version: 1,
        root: root.clone(),
        slot,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object: objects::reference(original)?,
    })?;
    Ok((json, body))
}
