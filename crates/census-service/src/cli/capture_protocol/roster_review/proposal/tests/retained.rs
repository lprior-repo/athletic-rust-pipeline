use super::super::decode_packet;
use super::TestResult;
use census_crawl::participation_evidence::provider_school::ProviderSchoolPacketV1;
use census_crawl::participation_evidence::{
    RetainedBodyEncodingV1, RetainedHtmlCaptureV1, RetainedHttpProvenanceV1,
};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::RetainedCaptureRootSchemaV1;

const ROSTER_URL: &str = "https://mi.milesplit.com/teams/12439-detroit-catholic-central/roster";

fn controlled_transport_packet(
) -> Result<ProviderSchoolPacketV1, Box<dyn std::error::Error + Send + Sync>> {
    Ok(ProviderSchoolPacketV1::new(
        controlled_transport_capture("https://mi.milesplit.com/teams")?,
        controlled_transport_capture(ROSTER_URL)?,
    )?)
}

fn controlled_transport_capture(
    url: &str,
) -> Result<RetainedHtmlCaptureV1, Box<dyn std::error::Error + Send + Sync>> {
    let instant = chrono::DateTime::parse_from_rfc3339("2026-10-16T00:49:43Z")?.to_utc();
    Ok(RetainedHtmlCaptureV1::new(
        url,
        url,
        200,
        instant,
        b"<html>controlled transport without original source provenance</html>",
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![("Accept".into(), "text/html".into())],
            response_header_bytes: b"HTTP/2 200\r\ncontent-type: text/html; charset=UTF-8\r\n\r\n"
                .to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: instant,
        },
    )?)
}

#[test]
fn retained_raw_decode_preserves_packet_bytes_reference_and_capture_instant() -> TestResult {
    let original = controlled_transport_packet()?;
    let bytes = original.encode()?;
    let reference = original.reference()?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_bytes(17)?;
    grant.charge_steps(3)?;
    let before = grant.usage();
    let packet = decode_packet(
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
        &bytes,
        reference.as_str(),
        &mut grant,
    )?;
    check!(eq; packet.encode()?, bytes);
    check!(eq; serde_json::to_vec(&packet)?, bytes);
    check!(eq; packet.reference()?, reference);
    check!(eq; packet.roster().acquired_at()?, original.roster().acquired_at());
    check!(eq; packet.roster().requested_url(), ROSTER_URL);
    check!(eq; grant.usage(), before);
    grant.ensure_available()?;
    Ok(())
}

#[test]
fn retained_raw_decode_keeps_historical_whitespace_acceptance_and_canonical_reference() -> TestResult
{
    let original = controlled_transport_packet()?;
    let canonical = original.encode()?;
    let mut spaced = canonical.clone();
    spaced.push(b' ');
    check!(ProviderSchoolPacketV1::decode(&spaced).is_err());
    let packet = decode_packet(
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
        &spaced,
        original.reference()?.as_str(),
        &mut AttachmentReplayGrantV1::new(),
    )?;
    check!(eq; packet.encode()?, canonical);
    check!(eq; packet.reference()?, original.reference()?);
    Ok(())
}

#[test]
fn raw_bytes_cannot_be_relabelled_as_observed_and_owned_decode_is_precharged() -> TestResult {
    let original = controlled_transport_packet()?;
    let bytes = original.encode()?;
    let mut grant = AttachmentReplayGrantV1::new();
    let result = decode_packet(
        RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1,
        &bytes,
        original.reference()?.as_str(),
        &mut grant,
    );
    check!(result.is_err());
    check!(eq; grant.usage().encoded_bytes(), bytes.len().checked_mul(3).ok_or("decode byte count")?);
    check!(eq; grant.usage().traversal_steps(), 64);
    check!(grant.ensure_available().is_err());
    let failed_usage = grant.usage();
    check!(decode_packet(
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
        &bytes,
        original.reference()?.as_str(),
        &mut grant,
    )
    .is_err());
    check!(eq; grant.usage(), failed_usage);
    Ok(())
}

#[test]
fn malformed_observed_bytes_are_charged_before_parse_and_failed_grant_is_poisoned() -> TestResult {
    let bytes = b"not an observed packet";
    let mut grant = AttachmentReplayGrantV1::new();
    check!(decode_packet(
        RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1,
        bytes,
        &"a".repeat(64),
        &mut grant,
    )
    .is_err());
    check!(eq; grant.usage().encoded_bytes(), bytes.len().checked_mul(3).ok_or("decode byte count")?);
    check!(eq; grant.usage().traversal_steps(), 64);
    check!(grant.ensure_available().is_err());
    Ok(())
}

#[test]
fn owned_decode_step_reservation_refuses_before_parse_without_resetting_existing_grant(
) -> TestResult {
    let bytes = b"not an observed packet";
    let mut grant = AttachmentReplayGrantV1::new();
    let existing_steps = AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
        .checked_sub(63)
        .ok_or("step precharge")?;
    grant.charge_steps(existing_steps)?;
    check!(decode_packet(
        RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1,
        bytes,
        &"a".repeat(64),
        &mut grant,
    )
    .is_err());
    check!(eq; grant.usage().traversal_steps(), existing_steps);
    check!(eq; grant.usage().encoded_bytes(), bytes.len().checked_mul(3).ok_or("decode byte count")?);
    check!(grant.ensure_available().is_err());
    Ok(())
}

#[test]
fn root_key_mismatch_refuses_valid_raw_packet_and_poisoned_grant_cannot_retry() -> TestResult {
    let original = controlled_transport_packet()?;
    let bytes = original.encode()?;
    let mut grant = AttachmentReplayGrantV1::new();
    let before = grant.usage();
    check!(decode_packet(
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
        &bytes,
        &"a".repeat(64),
        &mut grant,
    )
    .is_err());
    check!(grant.ensure_available().is_err());
    check!(decode_packet(
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
        &bytes,
        original.reference()?.as_str(),
        &mut grant,
    )
    .is_err());
    check!(eq; grant.usage(), before);
    Ok(())
}

#[test]
fn unrelated_capture_root_schema_refuses_without_reinterpreting_raw_packet() -> TestResult {
    let original = controlled_transport_packet()?;
    let bytes = original.encode()?;
    for schema in [
        RetainedCaptureRootSchemaV1::ProviderTargetPacketV1,
        RetainedCaptureRootSchemaV1::ParticipationPacketV1,
        RetainedCaptureRootSchemaV1::HtmlResultCaptureV1,
    ] {
        let mut grant = AttachmentReplayGrantV1::new();
        let before = grant.usage();
        check!(decode_packet(schema, &bytes, original.reference()?.as_str(), &mut grant).is_err());
        check!(grant.ensure_available().is_err());
        check!(eq; grant.usage(), before);
    }
    Ok(())
}
