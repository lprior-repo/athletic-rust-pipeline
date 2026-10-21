use super::*;

#[test]
fn bound_read_returns_the_exact_approved_inline_bytes() -> TestResult {
    let case = committed("active-inline-bound-exact")?;
    let object = case
        .context
        .objects
        .first()
        .ok_or("synthetic inline object absent")?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let bound = owner
        .bind_exact_subject(&case.context.subject, &mut grant)?
        .ok_or("active subject absent")?;
    check!(matches!(
        bound.read_context_object_v1(&object.reference, &mut grant)?,
        Some(crate::ReadAttachmentContextObjectV1::Inline { reference, bytes })
            if reference == &object.reference && bytes.as_slice() == object.bytes.as_slice()
    ));
    Ok(())
}

#[test]
fn bound_read_returns_none_for_an_object_introduced_by_another_active_record() -> TestResult {
    let case = committed("active-inline-bound-cross-record")?;
    let later = case.fixture.second_birth()?;
    let other = Context::synthetic("active-inline-bound-cross-other")?;
    let inspection = inspect_first(&case.fixture.store, later.public_id())?;
    let command = other.command("active-inline-bound-cross-other-attach", &inspection)?;
    written(apply(
        &case.fixture.store,
        prepare(&case.fixture.store, inspection, command, &other)?,
    )?)?;
    let other_object = other
        .objects
        .first()
        .ok_or("other synthetic inline object absent")?;

    let listed_view = case.fixture.store.snapshot();
    let mut listed_grant = AttachmentReplayGrantV1::new();
    let listed = listed_view.read_evidence_subject_attachment_references_v1(&mut listed_grant)?;
    check!(listed.records().iter().any(|record| {
        record
            .command
            .reviewed_context
            .objects
            .as_slice()
            .contains(&other_object.reference)
    }));

    let before = retained_state(&case.fixture.store)?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let bound = owner
        .bind_exact_subject(&case.context.subject, &mut grant)?
        .ok_or("original active subject absent")?;
    check!(eq; bound.record(), &case.record);
    check!(bound
        .read_context_object_v1(&other_object.reference, &mut grant)?
        .is_none());
    let other_bound = owner
        .bind_exact_subject(&other.subject, &mut grant)?
        .ok_or("other active subject absent")?;
    check!(matches!(
        other_bound.read_context_object_v1(&other_object.reference, &mut grant)?,
        Some(crate::ReadAttachmentContextObjectV1::Inline { reference, bytes })
            if reference == &other_object.reference
                && bytes.as_slice() == other_object.bytes.as_slice()
    ));
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn bound_read_refuses_a_wrong_length_reference_with_the_same_digest_and_poisons() -> TestResult {
    let case = committed("active-inline-bound-wrong-length")?;
    let approved = case
        .context
        .objects
        .first()
        .ok_or("synthetic inline object absent")?;
    let mut wrong_length = approved.reference.clone();
    wrong_length.bytes = wrong_length
        .bytes
        .checked_add(1)
        .ok_or("inline object length overflow")?;
    check!(eq; wrong_length.digest, approved.reference.digest);
    check!(ne; wrong_length.bytes, approved.reference.bytes);

    let before = retained_state(&case.fixture.store)?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let bound = owner
        .bind_exact_subject(&case.context.subject, &mut grant)?
        .ok_or("active subject absent")?;
    check!(matches!(
        bound.read_context_object_v1(&wrong_length, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(_)))
    ));
    poisoned(&mut grant)?;
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn bound_read_refuses_a_foreign_continuation_grant_without_changing_state() -> TestResult {
    let case = committed("active-inline-bound-foreign-grant")?;
    let object = case
        .context
        .objects
        .first()
        .ok_or("synthetic inline object absent")?;
    let before = retained_state(&case.fixture.store)?;
    let view = case.fixture.store.snapshot();
    let mut original = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut original)?;
    let bound = owner
        .bind_exact_subject(&case.context.subject, &mut original)?
        .ok_or("active subject absent")?;
    let mut foreign = AttachmentReplayGrantV1::new();
    check!(matches!(
        bound.read_context_object_v1(&object.reference, &mut foreign),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Invalid(_)
        )))
    ));
    poisoned(&mut foreign)?;
    check!(matches!(
        bound.read_context_object_v1(&object.reference, &mut original)?,
        Some(crate::ReadAttachmentContextObjectV1::Inline { reference, bytes })
            if reference == &object.reference && bytes.as_slice() == object.bytes.as_slice()
    ));
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}
