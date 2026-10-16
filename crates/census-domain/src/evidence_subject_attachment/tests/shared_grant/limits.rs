use super::*;

#[test]
fn shared_byte_and_step_limits_are_exact_and_poison_later_admission() -> TestResult {
    let mut bytes = AttachmentReplayGrantV1::new();
    bytes.charge_bytes(ReplayLimits::SMALL.total_text_bytes)?;
    check!(matches!(
        bytes.charge_bytes(1),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate encoded bytes",
            needed: 4_194_305,
            available: 4_194_304,
        })
    ));
    check!(eq;
        bytes.usage().encoded_bytes(),
        ReplayLimits::SMALL.total_text_bytes
    );
    check!(matches!(
        bytes.charge_steps(0),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    let mut steps = AttachmentReplayGrantV1::new();
    steps.charge_steps(ReplayLimits::SMALL.traversal_steps)?;
    check!(matches!(
        steps.charge_steps(1),
        Err(EvidenceSubjectAttachmentError::Capacity {
            resource: "aggregate traversal steps",
            needed: 8_193,
            available: 8_192,
        })
    ));
    check!(eq;
        steps.usage().traversal_steps(),
        ReplayLimits::SMALL.traversal_steps
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
