use super::*;

#[test]
fn active_inline_context_returns_exact_approved_bytes() -> TestResult {
    let case = committed("active-inline-context")?;
    let object = case
        .context
        .objects
        .first()
        .ok_or("synthetic inline object absent")?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(matches!(
        owner.read_active_context_object_v1(
            &case.context.subject,
            &object.reference,
            &mut grant,
        )?,
        Some(crate::ReadAttachmentContextObjectV1::Inline { reference, bytes })
            if reference == &object.reference && bytes.as_slice() == object.bytes.as_slice()
    ));
    Ok(())
}

#[test]
fn active_inline_context_refuses_wrong_length_and_returns_none_for_unapproved_digest() -> TestResult
{
    let case = committed("active-inline-context-authorization")?;
    let approved = case
        .context
        .objects
        .first()
        .ok_or("synthetic inline object absent")?;
    let view = case.fixture.store.snapshot();
    let mut owner_grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut owner_grant)?;

    let mut wrong_length = approved.reference.clone();
    wrong_length.bytes = wrong_length
        .bytes
        .checked_add(1)
        .ok_or("inline object length overflow")?;
    check!(matches!(
        owner
            .read_active_context_object_v1(&case.context.subject, &wrong_length, &mut owner_grant,),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(_)))
    ));
    poisoned(&mut owner_grant)?;

    let unapproved = InlineObject::synthetic("active-inline-context-unapproved")?;
    let mut fresh_grant = AttachmentReplayGrantV1::new();
    let fresh_owner =
        view.read_current_evidence_subject_attachment_references_v1(&mut fresh_grant)?;
    check!(fresh_owner
        .read_active_context_object_v1(
            &case.context.subject,
            &unapproved.reference,
            &mut fresh_grant,
        )?
        .is_none());
    Ok(())
}

#[test]
fn active_inline_context_is_absent_after_subject_revocation() -> TestResult {
    let case = committed("active-inline-context-revoked")?;
    let inspection = case.fixture.store.inspect_evidence_subject_attachment_v1(
        AttachmentInspectionRequestV1::Revoke {
            prior: case.record.reference(),
        },
    )?;
    let command = next_command(&case.record, "active-inline-context-revoke", &inspection)?;
    written(apply(
        &case.fixture.store,
        case.fixture.store.prepare_evidence_subject_attachment_v1(
            command,
            Preparation::New {
                inspection,
                context: &[],
            },
        )?,
    )?)?;

    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(owner
        .read_active_context_object_v1(
            &case.context.subject,
            &case
                .context
                .objects
                .first()
                .ok_or("object absent")?
                .reference,
            &mut grant,
        )?
        .is_none());
    Ok(())
}

#[test]
fn active_inline_context_rejects_a_foreign_grant_and_poison_is_not_recoverable() -> TestResult {
    let case = committed("active-inline-context-foreign-grant")?;
    let object = case
        .context
        .objects
        .first()
        .ok_or("synthetic inline object absent")?;
    let view = case.fixture.store.snapshot();
    let mut original = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut original)?;
    let mut foreign = AttachmentReplayGrantV1::new();
    check!(matches!(
        owner
            .read_active_context_object_v1(&case.context.subject, &object.reference, &mut foreign,),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Invalid(_)
        )))
    ));
    poisoned(&mut foreign)?;
    check!(matches!(
        owner.read_active_context_object_v1(
            &case.context.subject,
            &object.reference,
            &mut original,
        )?,
        Some(crate::ReadAttachmentContextObjectV1::Inline { reference, bytes })
            if reference == &object.reference && bytes.as_slice() == object.bytes.as_slice()
    ));
    Ok(())
}
