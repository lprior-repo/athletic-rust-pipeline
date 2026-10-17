use super::*;

#[test]
fn nonempty_current_owner_rejects_and_poisons_a_substituted_binding_grant() -> TestResult {
    let case = committed("current-foreign-binding-grant")?;
    let view = case.fixture.store.snapshot();
    let mut original = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut original)?;
    let mut foreign = AttachmentReplayGrantV1::new();
    check!(matches!(
        owner.bind_exact_subject(&case.context.subject, &mut foreign),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Invalid(_)
        )))
    ));
    poisoned(&mut foreign)?;
    let bound = owner
        .bind_exact_subject(&case.context.subject, &mut original)?
        .ok_or("original grant binding absent")?;
    bound.require_original_grant(&mut original)?;
    Ok(())
}

#[test]
fn borrowed_capability_requires_the_original_grant_at_proof_consumption() -> TestResult {
    let case = committed("current-foreign-consumer-grant")?;
    let view = case.fixture.store.snapshot();
    let mut original = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut original)?;
    let bound = owner
        .bind_exact_subject(&case.context.subject, &mut original)?
        .ok_or("active subject absent")?;
    let mut foreign = AttachmentReplayGrantV1::new();
    let usage = original.usage();
    foreign.charge_bytes(usage.encoded_bytes())?;
    foreign.charge_steps(usage.traversal_steps())?;
    check!(matches!(
        bound.require_original_grant(&mut foreign),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Invalid(_)
        )))
    ));
    poisoned(&mut foreign)?;
    check!(eq; bound.record(), &case.record);
    bound.require_original_grant(&mut original)?;
    Ok(())
}

#[test]
fn current_owner_and_capability_cannot_revive_a_poisoned_original_grant() -> TestResult {
    let case = committed("current-original-poison")?;
    let before = retained_state(&case.fixture.store)?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let bound = owner
        .bind_exact_subject(&case.context.subject, &mut grant)?
        .ok_or("active subject absent")?;
    grant.poison();
    check!(matches!(
        bound.require_original_grant(&mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Poisoned
        )))
    ));
    check!(matches!(
        owner.bind_exact_subject(&case.context.subject, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Poisoned
        )))
    ));
    check!(matches!(
        view.read_current_evidence_subject_attachment_references_v1(&mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Poisoned
        )))
    ));
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn empty_unbound_current_owner_preserves_fresh_grant_absence_without_issuing_authority(
) -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("current-empty-query")?;
    let view = fixture.store.snapshot();
    let mut original = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut original)?;
    let mut foreign = AttachmentReplayGrantV1::new();
    check!(owner
        .bind_exact_subject(&context.subject, &mut foreign)?
        .is_none());
    Ok(())
}
