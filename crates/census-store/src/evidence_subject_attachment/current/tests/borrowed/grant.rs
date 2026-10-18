use super::*;

const MAX_STEPS: usize = AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS;

fn measure_borrowed_constructor_steps() -> TestResult<usize> {
    let fixture = Fixture::new()?;
    let view = fixture.store.snapshot();
    let accepted = accepted(&view)?;
    let mut grant = AttachmentReplayGrantV1::new();
    view.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &accepted, &mut grant,
    )?;
    Ok(grant.usage().traversal_steps())
}

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
            EvidenceSubjectAttachmentError::Invalid(_)
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
            EvidenceSubjectAttachmentError::Invalid(_)
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
    poisoned(&mut grant)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn borrowed_constructor_exact_work_limit_completes_the_held_owner_comparison() -> TestResult {
    let fixture = Fixture::new()?;
    let before = retained_state(&fixture.store)?;
    let constructor_steps = measure_borrowed_constructor_steps()?;
    let view = fixture.store.snapshot();
    let accepted = accepted(&view)?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_steps(
        MAX_STEPS
            .checked_sub(constructor_steps)
            .ok_or("constructor work bound")?,
    )?;
    let owner = view.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &accepted, &mut grant,
    )?;
    check!(eq; grant.usage().traversal_steps(), MAX_STEPS);
    check!(std::ptr::eq(owner.accepted.view(), &accepted));
    grant.charge_steps(0)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn borrowed_constructor_one_step_short_refuses_the_final_held_owner_comparison() -> TestResult {
    let fixture = Fixture::new()?;
    let before = retained_state(&fixture.store)?;
    let constructor_steps = measure_borrowed_constructor_steps()?;
    let view = fixture.store.snapshot();
    let accepted = accepted(&view)?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_steps(
        MAX_STEPS
            .checked_sub(constructor_steps)
            .and_then(|value| value.checked_add(1))
            .ok_or("constructor work bound")?,
    )?;
    check!(matches!(
        view.read_current_evidence_subject_attachment_references_with_accepted_v1(
            &accepted, &mut grant
        ),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate traversal steps",
                needed: n,
                available,
            }
        ))) if available == MAX_STEPS && n.checked_sub(available) == Some(1)
    ));
    poisoned(&mut grant)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}
