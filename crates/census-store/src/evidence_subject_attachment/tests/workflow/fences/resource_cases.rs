use super::*;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;

pub(super) fn a_held_structural_read_poisoned_by_step_exhaustion_never_resets_the_original_grant(
) -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("step-exhaustion")?;
    let inspection = fixture.inspect()?;
    let command = context.command("step-exhaustion", &inspection)?;
    written(apply(
        &fixture.store,
        prepare(&fixture.store, inspection, command, &context)?,
    )?)?;
    let before = retained_state(&fixture.store)?;
    let held = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_steps(AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS)?;
    let usage = grant.usage();
    check!(matches!(
        held.read_evidence_subject_attachment_references_v1(&mut grant),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Domain(EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate traversal steps",
                needed,
                available,
            })
        )) if available == AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
            && needed.checked_sub(available) == Some(1)
    ));
    check!(eq; grant.usage(), usage);
    check!(matches!(
        held.read_evidence_subject_attachment_references_v1(&mut grant),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Domain(EvidenceSubjectAttachmentError::Poisoned)
        ))
    ));
    check!(eq; grant.usage(), usage);
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

pub(super) fn a_held_structural_read_poisoned_by_byte_exhaustion_preserves_original_charge_and_store(
) -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("byte-exhaustion")?;
    let inspection = fixture.inspect()?;
    let command = context.command("byte-exhaustion", &inspection)?;
    written(apply(
        &fixture.store,
        prepare(&fixture.store, inspection, command, &context)?,
    )?)?;
    let before = retained_state(&fixture.store)?;
    let held = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_bytes(AttachmentReplayGrantV1::MAX_ENCODED_BYTES)?;
    check!(
        matches!(held.read_evidence_subject_attachment_references_v1(&mut grant),
            Err(StoreError::EvidenceSubjectAttachment(AttachmentError::Domain(
                EvidenceSubjectAttachmentError::Capacity { resource: "aggregate encoded bytes", needed, available }
            ))) if available == AttachmentReplayGrantV1::MAX_ENCODED_BYTES && needed > available
        )
    );
    let usage = grant.usage();
    check!(eq; usage.encoded_bytes(), AttachmentReplayGrantV1::MAX_ENCODED_BYTES);
    check!(matches!(
        held.read_evidence_subject_attachment_references_v1(&mut grant),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Domain(EvidenceSubjectAttachmentError::Poisoned)
        ))
    ));
    check!(eq; grant.usage(), usage);
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}
