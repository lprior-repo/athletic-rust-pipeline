use super::*;
use census_domain::public_identity_history::ReplayLimits;

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
    grant.charge_steps(ReplayLimits::SMALL.traversal_steps)?;
    let usage = grant.usage();
    check!(matches!(
        held.read_evidence_subject_attachment_references_v1(&mut grant),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Domain(EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate traversal steps",
                needed: 8_193,
                available: 8_192,
            })
        ))
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
    grant.charge_bytes(ReplayLimits::SMALL.total_text_bytes)?;
    check!(
        matches!(held.read_evidence_subject_attachment_references_v1(&mut grant),
            Err(StoreError::EvidenceSubjectAttachment(AttachmentError::Domain(
                EvidenceSubjectAttachmentError::Capacity { resource: "aggregate encoded bytes", needed, available: 4_194_304 }
            ))) if needed > 4_194_304
        )
    );
    let usage = grant.usage();
    check!(eq; usage.encoded_bytes(), 4_194_304);
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
