use super::*;

#[test]
fn borrowed_current_owner_and_capability_require_the_original_bound_grant() -> TestResult {
    let case = committed("borrowed-current-grant")?;
    let before = retained_state(&case.fixture.store)?;
    let view = case.fixture.store.snapshot();
    let accepted = accepted(&view)?;
    let mut original = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &accepted,
        &mut original,
    )?;
    let mut foreign = AttachmentReplayGrantV1::new();
    check!(matches!(
        owner.bind_exact_subject(&case.context.subject, &mut foreign),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Invalid("checked command grant differs")
        )))
    ));
    poisoned(&mut foreign)?;
    let bound = owner
        .bind_exact_subject(&case.context.subject, &mut original)?
        .ok_or("original subject absent")?;
    let mut consumer = AttachmentReplayGrantV1::new();
    check!(matches!(
        bound.require_original_grant(&mut consumer),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Invalid("checked command grant differs")
        )))
    ));
    poisoned(&mut consumer)?;
    bound.require_original_grant(&mut original)?;
    check!(eq; bound.record(), &case.record);
    check!(eq; bound.receipt(), &case.receipt);
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn borrowed_constructor_cannot_revive_a_poisoned_grant() -> TestResult {
    let fixture = Fixture::new()?;
    let before = retained_state(&fixture.store)?;
    let view = fixture.store.snapshot();
    let accepted = accepted(&view)?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.poison();
    check!(matches!(
        view.read_current_evidence_subject_attachment_references_with_accepted_v1(
            &accepted, &mut grant
        ),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Poisoned
        )))
    ));
    check!(eq; grant.usage().traversal_steps(), 0);
    check!(eq; grant.usage().encoded_bytes(), 0);
    poisoned(&mut grant)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn borrowed_constructor_exact_work_limit_completes_the_held_owner_comparison() -> TestResult {
    let fixture = Fixture::new()?;
    let before = retained_state(&fixture.store)?;
    let view = fixture.store.snapshot();
    let accepted = accepted(&view)?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_steps(8_180)?;
    let owner = view.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &accepted, &mut grant,
    )?;
    check!(eq; grant.usage().traversal_steps(), 8_192);
    check!(eq; grant.usage().encoded_bytes(), 0);
    check!(std::ptr::eq(owner.accepted.view(), &accepted));
    grant.charge_steps(0)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn borrowed_constructor_one_step_short_refuses_the_final_held_owner_comparison() -> TestResult {
    let fixture = Fixture::new()?;
    let before = retained_state(&fixture.store)?;
    let view = fixture.store.snapshot();
    let accepted = accepted(&view)?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_steps(8_181)?;
    check!(matches!(
        view.read_current_evidence_subject_attachment_references_with_accepted_v1(
            &accepted, &mut grant
        ),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate traversal steps",
                needed: 8_193,
                available: 8_192,
            }
        )))
    ));
    check!(eq; grant.usage().traversal_steps(), 8_192);
    poisoned(&mut grant)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}
