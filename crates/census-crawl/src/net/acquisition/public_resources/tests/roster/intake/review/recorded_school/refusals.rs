use super::*;
use crate::participation_evidence::recorded::decode_provider_school_packet;
use base64::{engine::general_purpose::STANDARD, Engine};
use census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError;

fn poisoned(grant: &AttachmentReplayGrantV1) -> TestResult {
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

fn corrupted(held: &Held) -> TestResult {
    for (pointer, replacement, expected) in [
        (
            "/directory/body",
            STANDARD.encode(b"changed original"),
            "public school intake original body differs",
        ),
        (
            "/roster/terminal_receipt/digest",
            "0".repeat(64),
            "public school intake terminal proof differs",
        ),
    ] {
        let mut wire: serde_json::Value = serde_json::from_slice(&held.bytes)?;
        *wire
            .pointer_mut(pointer)
            .ok_or("actual immutable packet field")? = replacement.into();
        let mut changed = Vec::new();
        census_domain::model::write_serialized(&mut changed, &wire)?;
        let mut grant = AttachmentReplayGrantV1::new();
        let error = decode_provider_school_packet(held.root.schema, &changed, &mut grant)
            .err()
            .ok_or("corrupt owned packet accepted")?;
        check!(
            matches!(error, crate::CrawlError::Store(StoreError::Refused { detail })
            if detail == expected)
        );
        check!(eq; grant.usage().encoded_bytes(), changed.len().checked_mul(3).ok_or("decode bytes")?);
        check!(eq; grant.usage().traversal_steps(), 64);
        poisoned(&grant)?;
        check!(decode_provider_school_packet(held.root.schema, &held.bytes, &mut grant).is_err());
    }
    Ok(())
}

fn exhausted(original: &RetainedCaptureRootReadV1, held: &Held) -> TestResult {
    let required = held.bytes.len().checked_mul(3).ok_or("decode bytes")?;
    let mut bytes = AttachmentReplayGrantV1::new();
    let padding = AttachmentReplayGrantV1::MAX_ENCODED_BYTES
        .checked_sub(required)
        .and_then(|n| n.checked_add(1))
        .ok_or("decode padding")?;
    bytes.charge_bytes(padding)?;
    check!(read_original_provider_school(original, &mut bytes).is_err());
    check!(eq; bytes.usage().encoded_bytes(), padding);
    check!(eq; bytes.usage().traversal_steps(), 0);
    poisoned(&bytes)?;
    let mut steps = AttachmentReplayGrantV1::new();
    let padding = AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
        .checked_sub(63)
        .ok_or("step padding")?;
    steps.charge_steps(padding)?;
    check!(read_original_provider_school(original, &mut steps).is_err());
    check!(eq; steps.usage().encoded_bytes(), required);
    check!(eq; steps.usage().traversal_steps(), padding);
    poisoned(&steps)?;
    Ok(())
}

#[tokio::test]
async fn actual_owned_root_refuses_corrupt_proof_and_exhausted_original_grants_without_writes(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = review_fixture(&listener)?;
    let (directory, roster) = acquire_pair(&fixture, listener, "text/html").await?;
    let held = mint(&fixture, &directory, &roster)?;
    let original = root(&fixture.store, &held)?;
    let before = Witness::capture(&fixture.store)?;
    corrupted(&held)?;
    exhausted(&original, &held)?;
    check!(eq; Witness::capture(&fixture.store)?, before);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 3);
    Ok(())
}
