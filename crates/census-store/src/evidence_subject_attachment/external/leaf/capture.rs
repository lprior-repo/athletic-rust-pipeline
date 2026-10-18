use super::{schema, Error};
use crate::{StoreError, StoreResult};
use base64::{engine::general_purpose::STANDARD, Engine};
use census_domain::evidence_subject_attachment::{
    AttachmentObjectReferenceV1, AttachmentReplayGrantV1, EvidenceSubjectAttachmentError,
};
use serde::Deserialize;
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};

const MAX_BODY_BYTES: usize = 512 * 1024;
const MAX_BASE64_BYTES: usize = 699_052;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Capture<'a> {
    #[serde(rename = "requested_url", borrow)]
    _requested_url: &'a RawValue,
    #[serde(rename = "final_url", borrow)]
    _final_url: &'a RawValue,
    #[serde(rename = "status", borrow)]
    _status: &'a RawValue,
    #[serde(rename = "acquired_at", borrow)]
    _acquired_at: &'a RawValue,
    body_bytes: usize,
    body_sha256: &'a str,
    #[serde(borrow)]
    body_base64: &'a RawValue,
    #[serde(borrow)]
    provenance: &'a RawValue,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Provenance<'a> {
    #[serde(rename = "request_method", borrow)]
    _request_method: &'a RawValue,
    #[serde(rename = "request_headers", borrow)]
    _request_headers: &'a RawValue,
    #[serde(rename = "response_header_bytes", borrow)]
    _response_header_bytes: &'a RawValue,
    #[serde(rename = "content_type", borrow)]
    _content_type: &'a RawValue,
    #[serde(rename = "body_encoding", borrow)]
    _body_encoding: &'a RawValue,
    #[serde(rename = "started_at", borrow)]
    _started_at: &'a RawValue,
}

pub(super) fn validate(raw: &RawValue, grant: &mut AttachmentReplayGrantV1) -> StoreResult<()> {
    checked(raw, grant).map(|_| ())
}

pub(crate) struct CheckedCapture<'a> {
    raw: &'a RawValue,
    requested_url: &'a RawValue,
    pub(crate) body_sha256: &'a str,
    body_bytes: usize,
    encoded: &'a str,
}

pub(in crate::evidence_subject_attachment::external) fn checked<'a>(
    raw: &'a RawValue,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<CheckedCapture<'a>> {
    let capture: Capture<'a> = schema::parse(raw, grant)?;
    let _provenance: Provenance<'_> = schema::parse(capture.provenance, grant)?;
    from_capture(capture, raw, grant)
}

fn from_capture<'a>(
    capture: Capture<'a>,
    raw: &'a RawValue,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<CheckedCapture<'a>> {
    let encoded = encoded(&capture, grant)?;
    Ok(CheckedCapture {
        raw,
        requested_url: capture._requested_url,
        body_sha256: capture.body_sha256,
        body_bytes: capture.body_bytes,
        encoded,
    })
}

fn encoded<'a>(capture: &Capture<'a>, grant: &mut AttachmentReplayGrantV1) -> StoreResult<&'a str> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    let raw = capture.body_base64.get();
    if raw.len() > MAX_BASE64_BYTES + 2
        || capture.body_bytes == 0
        || capture.body_bytes > MAX_BODY_BYTES
    {
        return Err(Error::Corrupt("external capture body bounds").into());
    }
    let text = raw
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or(Error::Corrupt(
            "external capture body is not a plain base64 string",
        ))?;
    let expected = capture
        .body_bytes
        .checked_add(2)
        .and_then(|n| (n / 3).checked_mul(4))
        .ok_or(StoreError::CounterOverflow)?;
    if text.len() != expected
        || !text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
        || capture.body_sha256.len() != 64
        || !capture
            .body_sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Corrupt("external capture base64 or digest shape").into());
    }
    Ok(text)
}

impl CheckedCapture<'_> {
    pub(crate) fn requested_url(
        &self,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<std::borrow::Cow<'_, str>> {
        let raw = self.requested_url.get();
        let inner = raw
            .strip_prefix('"')
            .and_then(|text| text.strip_suffix('"'))
            .ok_or(Error::Corrupt("external capture requested URL type"))?;
        if inner.contains('\\') {
            grant.charge_bytes(raw.len()).map_err(Error::Domain)?;
            serde_json::from_str(raw)
                .map(std::borrow::Cow::Owned)
                .map_err(|_| Error::Corrupt("external capture requested URL").into())
        } else {
            Ok(std::borrow::Cow::Borrowed(inner))
        }
    }

    pub(in crate::evidence_subject_attachment::external) fn raw(&self) -> &RawValue {
        self.raw
    }

    pub(crate) fn body_bytes(
        &self,
        reference: &AttachmentObjectReferenceV1,
        grant: &mut AttachmentReplayGrantV1,
    ) -> StoreResult<Vec<u8>> {
        if reference.bytes
            != u64::try_from(self.body_bytes).map_err(|_| StoreError::CounterOverflow)?
            || reference.digest.as_str() != self.body_sha256
        {
            return Err(Error::Corrupt("external body reference differs from capture").into());
        }
        grant.charge_steps(2).map_err(Error::Domain)?;
        grant.charge_bytes(self.body_bytes).map_err(Error::Domain)?;
        let mut decoded = Vec::new();
        decoded
            .try_reserve_exact(self.body_bytes)
            .map_err(|_| Error::Domain(EvidenceSubjectAttachmentError::Allocation))?;
        decoded.resize(self.body_bytes, 0);
        let actual = STANDARD
            .decode_slice(self.encoded, &mut decoded)
            .map_err(|_| Error::Corrupt("external capture body base64 is invalid"))?;
        if actual != self.body_bytes {
            return Err(Error::Corrupt("external body decoded length").into());
        }
        require_reference(&decoded, reference, grant)?;
        Ok(decoded)
    }
}

pub(super) fn require_reference(
    raw: &[u8],
    reference: &AttachmentObjectReferenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    if u64::try_from(raw.len()).map_err(|_| StoreError::CounterOverflow)? != reference.bytes
        || format!("{:x}", Sha256::digest(raw)) != reference.digest.as_str()
    {
        return Err(Error::Corrupt("external leaf reference differs").into());
    }
    Ok(())
}
