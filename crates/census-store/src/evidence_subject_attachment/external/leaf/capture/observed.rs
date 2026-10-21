use super::{schema, CheckedCapture, Error, MAX_BASE64_BYTES, MAX_BODY_BYTES};
use crate::evidence_subject_attachment::external::leaf::{
    schema::CheckedSchool, RetainedCaptureSlotV1 as Slot,
};
use crate::evidence_subject_attachment::external::MAX_RETAINED_CAPTURE_ENVELOPE_BYTES;
use crate::{PublicSchoolIntakeCaptureV1, PublicSchoolIntakePacketV1, StoreError, StoreResult};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use serde::Deserialize;
use serde_json::value::RawValue;

#[derive(Deserialize)]
struct School<'a> {
    #[serde(borrow)]
    directory: &'a RawValue,
    #[serde(borrow)]
    roster: &'a RawValue,
}

#[derive(Deserialize)]
struct Capture<'a> {
    #[serde(borrow)]
    work: &'a RawValue,
    #[serde(borrow)]
    prepared: &'a RawValue,
    #[serde(borrow)]
    body: &'a RawValue,
}

#[derive(Deserialize)]
struct Work<'a> {
    #[serde(borrow)]
    request: &'a RawValue,
}

#[derive(Deserialize)]
struct Request<'a> {
    #[serde(borrow)]
    original_url: &'a RawValue,
}

#[derive(Deserialize)]
struct Prepared<'a> {
    #[serde(borrow)]
    capture: &'a RawValue,
}

#[derive(Deserialize)]
struct Content<'a> {
    content_digest: &'a str,
    content_bytes: usize,
    #[serde(borrow)]
    metadata_bytes: &'a RawValue,
}

struct Borrowed<'a> {
    capture: CheckedCapture<'a>,
    encoded_bytes: usize,
    metadata_bytes: usize,
}

pub(in crate::evidence_subject_attachment::external) fn school<'a>(
    raw: &'a RawValue,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<CheckedSchool<'a>> {
    grant.ensure_available().map_err(Error::Domain)?;
    if raw.get().len() > MAX_RETAINED_CAPTURE_ENVELOPE_BYTES {
        return Err(Error::Corrupt("observed school root bytes").into());
    }
    let school: School<'a> = schema::parse(raw, grant)?;
    let directory = borrowed(school.directory, grant)?;
    let roster = borrowed(school.roster, grant)?;
    charge_decode(raw, &directory, &roster, grant)?;
    let packet = PublicSchoolIntakePacketV1::decode(raw.get().as_bytes())?;
    require_capture(&directory.capture, packet.directory(), grant)?;
    require_capture(&roster.capture, packet.roster(), grant)?;
    Ok(CheckedSchool {
        directory: directory.capture,
        roster: roster.capture,
    })
}

pub(in crate::evidence_subject_attachment::external) fn select<'a>(
    raw: &'a RawValue,
    slot: Slot,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<&'a RawValue> {
    let school = school(raw, grant)?;
    match slot {
        Slot::ProviderSchoolDirectory => Ok(school.directory.raw()),
        Slot::ProviderSchoolRoster => Ok(school.roster.raw()),
        _ => Err(Error::Corrupt("external root capture slot").into()),
    }
}

fn borrowed<'a>(
    raw: &'a RawValue,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<Borrowed<'a>> {
    let capture: Capture<'a> = schema::parse(raw, grant)?;
    let work: Work<'a> = schema::parse(capture.work, grant)?;
    let request: Request<'a> = schema::parse(work.request, grant)?;
    let prepared: Prepared<'a> = schema::parse(capture.prepared, grant)?;
    let content: Content<'a> = schema::parse(prepared.capture, grant)?;
    let encoded = encoded(capture.body, &content, grant)?;
    Ok(Borrowed {
        capture: CheckedCapture {
            raw,
            requested_url: request.original_url,
            body_sha256: content.content_digest,
            body_bytes: content.content_bytes,
            encoded,
        },
        encoded_bytes: capture.body.get().len(),
        metadata_bytes: content.metadata_bytes.get().len(),
    })
}

fn encoded<'a>(
    raw: &'a RawValue,
    content: &Content<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<&'a str> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    if raw.get().len() > MAX_BASE64_BYTES + 2
        || content.content_bytes == 0
        || content.content_bytes > MAX_BODY_BYTES
    {
        return Err(Error::Corrupt("observed capture body bounds").into());
    }
    let text = raw
        .get()
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .ok_or(Error::Corrupt("observed capture body encoding"))?;
    let expected = content
        .content_bytes
        .checked_add(2)
        .and_then(|n| (n / 3).checked_mul(4))
        .ok_or(StoreError::CounterOverflow)?;
    if text.len() != expected
        || !text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
        || content.content_digest.len() != 64
        || !content
            .content_digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(Error::Corrupt("observed capture base64 or digest shape").into());
    }
    Ok(text)
}

fn charge_decode(
    raw: &RawValue,
    directory: &Borrowed<'_>,
    roster: &Borrowed<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let bytes = [directory, roster].into_iter().try_fold(
        raw.get()
            .len()
            .checked_mul(3)
            .ok_or(StoreError::CounterOverflow)?,
        |total, value| {
            let base64 = value
                .encoded_bytes
                .checked_mul(4)
                .ok_or(StoreError::CounterOverflow)?;
            let body = value
                .capture
                .body_bytes
                .checked_mul(2)
                .ok_or(StoreError::CounterOverflow)?;
            let metadata = value
                .metadata_bytes
                .checked_mul(3)
                .ok_or(StoreError::CounterOverflow)?;
            total
                .checked_add(base64)
                .and_then(|n| n.checked_add(body))
                .and_then(|n| n.checked_add(metadata))
                .ok_or(StoreError::CounterOverflow)
        },
    )?;
    grant.charge_steps(1).map_err(Error::Domain)?;
    grant.charge_bytes(bytes).map_err(Error::Domain)?;
    Ok(())
}

fn require_capture(
    capture: &CheckedCapture<'_>,
    actual: &PublicSchoolIntakeCaptureV1,
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    grant.charge_steps(1).map_err(Error::Domain)?;
    if capture.requested_url(grant)?.as_ref() != actual.work().request().original_url()
        || capture.body_sha256 != actual.prepared().content_digest().as_str()
        || u64::try_from(capture.body_bytes).map_err(|_| StoreError::CounterOverflow)?
            != actual.prepared().content_bytes()
        || capture.body_bytes != actual.body().len()
    {
        return Err(Error::Corrupt("observed capture literal differs from packet").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
