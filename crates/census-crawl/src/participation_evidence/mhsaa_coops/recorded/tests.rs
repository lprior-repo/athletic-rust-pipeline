use super::*;

mod fixture;
mod refusals;
mod roundtrip;

use super::super::packet::MHSAA_COOP_PDF_CAPTURE_PHASE;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn decode_of_genuine_raw_root_payload_keeps_original_reference_and_shared_debits() -> TestResult {
    let packet = super::super::packet::tests::packet()?;
    let bytes = packet.encode()?;
    let reference = packet.reference()?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_bytes(17)?;
    grant.charge_steps(3)?;
    let read = decode_payload(&bytes, &reference, &mut grant)?;
    check!(eq; read.reference()?, reference);
    check!(eq; read.capture().original_bytes(), packet.capture().original_bytes());
    check!(eq; grant.usage().encoded_bytes(), bytes.len().checked_mul(4).and_then(|n| n.checked_add(17)).ok_or("grant overflow")?);
    check!(eq; grant.usage().traversal_steps(), 4);
    Ok(())
}

#[test]
fn wrong_root_reference_or_corrupt_payload_poison_the_original_grant() -> TestResult {
    let packet = super::super::packet::tests::packet()?;
    let reference = MhsaaCoopPdfPacketRefV1::parse(&"0".repeat(64))?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(decode_payload(&packet.encode()?, &reference, &mut grant).is_err());
    check!(grant.ensure_available().is_err());
    let mut grant = AttachmentReplayGrantV1::new();
    check!(decode_payload(b"{}", &packet.reference()?, &mut grant).is_err());
    check!(grant.ensure_available().is_err());
    Ok(())
}

#[test]
fn exhausted_root_grant_refuses_without_binary_decode_or_new_pool() -> TestResult {
    let packet = super::super::packet::tests::packet()?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_bytes(AttachmentReplayGrantV1::MAX_ENCODED_BYTES)?;
    let before = grant.usage();
    check!(decode_payload(&packet.encode()?, &packet.reference()?, &mut grant).is_err());
    check!(eq; grant.usage(), before);
    check!(grant.ensure_available().is_err());
    Ok(())
}
