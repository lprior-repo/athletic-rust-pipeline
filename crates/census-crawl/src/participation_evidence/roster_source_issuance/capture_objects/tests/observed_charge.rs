use super::*;

#[test]
fn raw_precharge_keeps_exact_previous_usage_and_observed_decode_reserves_three_copies() -> TestResult
{
    for (schema, bytes, steps) in [
        (RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1, 128, 1),
        (
            RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1,
            384,
            64,
        ),
    ] {
        let mut grant = AttachmentReplayGrantV1::new();
        precharge_packet(schema, 128, &mut grant)?;
        check!(eq; grant.usage().encoded_bytes(), bytes);
        check!(eq; grant.usage().traversal_steps(), steps);
    }
    Ok(())
}

#[test]
fn observed_decode_precharge_refuses_bytes_or_steps_before_packet_allocation() -> TestResult {
    let schema = RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1;
    let mut bytes = AttachmentReplayGrantV1::new();
    bytes.charge_bytes(
        AttachmentReplayGrantV1::MAX_ENCODED_BYTES
            .checked_sub(383)
            .ok_or("byte ceiling")?,
    )?;
    let before = bytes.usage().encoded_bytes();
    check!(precharge_packet(schema, 128, &mut bytes).is_err());
    check!(eq; bytes.usage().encoded_bytes(), before);
    check!(bytes.ensure_available().is_err());
    let mut steps = AttachmentReplayGrantV1::new();
    steps.charge_steps(
        AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
            .checked_sub(63)
            .ok_or("step ceiling")?,
    )?;
    check!(precharge_packet(schema, 128, &mut steps).is_err());
    check!(steps.ensure_available().is_err());
    Ok(())
}
