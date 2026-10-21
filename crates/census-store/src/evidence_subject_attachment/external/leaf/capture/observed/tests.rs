use super::*;
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentObjectReferenceV1, EvidenceSubjectAttachmentError,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const BODY_SHA: &str = "2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881";
const RAW: &str = concat!(
    r#"{"requested_url":"https://example.test/source","final_url":"https://example.test/source","status":200,"acquired_at":"2026-10-16T02:00:00Z","body_bytes":1,"body_sha256":"#,
    r#""2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881","body_base64":"eA==","provenance":{"request_method":"GET","request_headers":[],"response_header_bytes":[],"content_type":"text/html; charset=utf-8","body_encoding":"identity","started_at":"2026-10-16T02:00:00Z"}}"#,
);

fn unproved_root() -> Result<Box<RawValue>, Box<dyn std::error::Error>> {
    let capture = format!(
        r#"{{"work":{{"request":{{"original_url":"https://example.test/source"}}}},"prepared":{{"capture":{{"content_digest":"{BODY_SHA}","content_bytes":1,"metadata_bytes":[1]}}}},"body":"eA=="}}"#,
    );
    Ok(RawValue::from_string(format!(
        r#"{{"directory":{capture},"roster":{capture}}}"#,
    ))?)
}

#[test]
fn existing_raw_capture_keeps_literal_hash_body_url_and_exact_grant_charges() -> TestResult {
    let raw = RawValue::from_string(RAW.to_string())?;
    let mut grant = AttachmentReplayGrantV1::new();
    let checked = super::super::checked(&raw, &mut grant)?;
    check!(eq; checked.raw().get(), RAW);
    check!(eq; checked.requested_url(&mut grant)?.as_ref(), "https://example.test/source");
    check!(eq; grant.usage().encoded_bytes(), 0);
    check!(eq; grant.usage().traversal_steps(), 3);
    checked.require_reference(
        &AttachmentObjectReferenceV1 {
            digest: AttachmentDigestV1::parse(
                "d9bf5a8da154affe9faa713f09e862c3aa76891b2dd032f6c31442aeb8426a46",
            )?,
            bytes: 447,
        },
        &mut grant,
    )?;
    let body = checked.body_bytes(
        &AttachmentObjectReferenceV1 {
            digest: AttachmentDigestV1::parse(BODY_SHA)?,
            bytes: 1,
        },
        &mut grant,
    )?;
    check!(eq; body.as_slice(), b"x");
    check!(eq; grant.usage().encoded_bytes(), 1);
    check!(eq; grant.usage().traversal_steps(), 7);
    Ok(())
}

#[test]
fn borrowed_observed_shape_requires_complete_immutable_packet_validation() -> TestResult {
    let raw = unproved_root()?;
    let mut grant = AttachmentReplayGrantV1::new();
    let error = school(&raw, &mut grant)
        .err()
        .ok_or("unproved shape became school authority")?;
    check!(matches!(error, StoreError::Refused { detail }
        if detail == "public school intake packet decoding"));
    let expected = raw
        .get()
        .len()
        .checked_mul(3)
        .and_then(|n| n.checked_add(70))
        .ok_or("fixture expected charge overflow")?;
    check!(eq; grant.usage().encoded_bytes(), expected);
    check!(eq; grant.usage().traversal_steps(), 14);
    Ok(())
}

#[test]
fn one_byte_short_original_grant_refuses_before_typed_packet_allocation_and_stays_poisoned(
) -> TestResult {
    let raw = unproved_root()?;
    let needed = raw
        .get()
        .len()
        .checked_mul(3)
        .and_then(|n| n.checked_add(70))
        .ok_or("fixture decode charge overflow")?;
    let precharge = AttachmentReplayGrantV1::MAX_ENCODED_BYTES
        .checked_sub(needed)
        .and_then(|n| n.checked_add(1))
        .ok_or("fixture precharge overflow")?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_bytes(precharge)?;
    check!(matches!(school(&raw, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate encoded bytes", needed, available,
            }
        ))) if available == AttachmentReplayGrantV1::MAX_ENCODED_BYTES
            && needed.checked_sub(available) == Some(1)));
    check!(eq; grant.usage().encoded_bytes(), precharge);
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(matches!(
        school(&raw, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Poisoned
        )))
    ));
    Ok(())
}
