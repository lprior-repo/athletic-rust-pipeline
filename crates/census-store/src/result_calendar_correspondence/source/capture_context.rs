use super::*;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Capture<'a> {
    #[serde(borrow)]
    acquired_at: &'a RawValue,
    #[serde(borrow)]
    body_base64: &'a RawValue,
    #[serde(borrow)]
    body_bytes: &'a RawValue,
    #[serde(borrow)]
    body_sha256: &'a RawValue,
    #[serde(borrow)]
    final_url: &'a RawValue,
    #[serde(borrow)]
    provenance: Provenance<'a>,
    #[serde(borrow)]
    requested_url: &'a RawValue,
    #[serde(borrow)]
    status: &'a RawValue,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Provenance<'a> {
    #[serde(borrow)]
    body_encoding: &'a RawValue,
    #[serde(borrow)]
    content_type: &'a RawValue,
    #[serde(borrow)]
    request_headers: &'a RawValue,
    #[serde(borrow)]
    request_method: &'a RawValue,
    #[serde(borrow)]
    response_header_bytes: &'a RawValue,
    #[serde(borrow)]
    started_at: &'a RawValue,
}

pub(super) fn require(
    raw: &RawValue,
    expected: &AttachmentObjectReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant.charge_bytes(raw.get().len()).map_err(grant_error)?;
    grant
        .charge_steps(raw.get().len().div_ceil(512).max(1))
        .map_err(grant_error)?;
    let capture: Capture<'_> = serde_json::from_str(raw.get())
        .map_err(|_| Error::SourceMismatch("original context capture schema"))?;
    let (digest, bytes) = codec::serialized_digest(&capture, grant)?;
    let bytes = u64::try_from(bytes).map_err(|_| Error::BudgetExceeded)?;
    if expected.bytes != bytes || expected.digest != digest {
        return Err(Error::SourceMismatch("original context capture").into());
    }
    Ok(())
}
