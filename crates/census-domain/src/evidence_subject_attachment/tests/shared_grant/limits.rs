use super::*;

#[test]
fn shared_byte_and_step_limits_are_exact_and_poison_later_admission() -> TestResult {
    let mut bytes = AttachmentReplayGrantV1::new();
    bytes.charge_bytes(AttachmentReplayGrantV1::MAX_ENCODED_BYTES)?;
    check!(matches!(
        bytes.charge_bytes(1),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate encoded bytes",
            needed,
            available,
        }) if available == AttachmentReplayGrantV1::MAX_ENCODED_BYTES
            && needed.checked_sub(available) == Some(1)
    ));
    check!(eq;
        bytes.usage().encoded_bytes(),
        AttachmentReplayGrantV1::MAX_ENCODED_BYTES
    );
    check!(matches!(
        bytes.charge_steps(0),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    let mut steps = AttachmentReplayGrantV1::new();
    steps.charge_steps(AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS)?;
    check!(matches!(
        steps.charge_steps(1),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate traversal steps",
            needed,
            available,
        }) if available == AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
            && needed.checked_sub(available) == Some(1)
    ));
    check!(eq;
        steps.usage().traversal_steps(),
        AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
    );
    check!(matches!(
        steps.charge_bytes(0),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn overflow_preserves_primary_error_and_cannot_be_rehabilitated() -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_bytes(1)?;
    check!(matches!(
        grant.charge_bytes(usize::MAX),
        Err(EvidenceSubjectAttachmentError::Overflow)
    ));
    check!(eq; grant.usage().encoded_bytes(), 1);
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(matches!(
        grant.charge_bytes(0),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}
