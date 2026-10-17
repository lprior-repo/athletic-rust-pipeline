use super::*;

#[test]
fn current_exact_active_subject_borrows_the_original_record_receipt_and_held_view() -> TestResult {
    let case = committed("current-exact-active")?;
    let before = retained_state(&case.fixture.store)?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(std::ptr::eq(owner.snapshot(), &view));
    let expected = case.context.subject.clone();
    let bound = owner
        .bind_exact_subject(&expected, &mut grant)?
        .ok_or("active subject absent")?;
    check!(!std::ptr::eq(bound.subject(), &expected));
    drop(expected);
    bound.require_original_grant(&mut grant)?;
    check!(std::ptr::eq(bound.snapshot(), &view));
    check!(eq; bound.subject(), &case.context.subject);
    check!(eq; bound.record(), &case.record);
    check!(eq; bound.receipt(), &case.receipt);
    check!(eq; bound.target(), &case.record.command.target);
    check!(eq; bound.store_identity(), &case.record.command.store);
    check!(eq; bound.accepted_public_head(), &case.fixture.writer.head(&case.fixture.store)?);
    let mut retained_grant = AttachmentReplayGrantV1::new();
    let retained = view.read_evidence_subject_attachment_references_v1(&mut retained_grant)?;
    check!(eq; bound.inventory(), retained.inventory());
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn different_subject_digest_is_explicit_absence_even_when_all_other_fields_match() -> TestResult {
    let case = committed("current-different-row")?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let mut expected = case.context.subject.clone();
    expected.subject = replacement_digest()?;
    check!(ne; expected.subject, case.context.subject.subject);
    check!(owner.bind_exact_subject(&expected, &mut grant)?.is_none());
    check!(owner
        .bind_exact_subject(&case.context.subject, &mut grant)?
        .is_some());
    Ok(())
}

#[test]
fn matching_subject_digest_cannot_hide_any_different_subject_metadata() -> TestResult {
    let case = committed("current-full-subject-equality")?;
    let original = &case.context.subject;
    let digest = replacement_digest()?;
    let text = AttachmentTextV1::parse("different-controlled-subject-metadata")?;
    let mut variants = Vec::new();
    for field in 0..9 {
        let mut candidate = original.clone();
        match field {
            0 => candidate.namespace = text.clone(),
            1 => candidate.native_id = Some(text.clone()),
            2 => candidate.native_id = None,
            3 => candidate.capture = digest.clone(),
            4 => candidate.body = digest.clone(),
            5 => candidate.interpretation = digest.clone(),
            6 => candidate.locator = digest.clone(),
            7 => candidate.context = digest.clone(),
            8 => candidate.role = AttachmentSubjectRoleV1::AthleteStatus,
            _ => return Err("controlled metadata field absent".into()),
        }
        check!(ne; &candidate, original);
        check!(eq; candidate.subject, original.subject);
        variants.push(candidate);
    }
    let before = retained_state(&case.fixture.store)?;
    let view = case.fixture.store.snapshot();
    for candidate in variants {
        let mut grant = AttachmentReplayGrantV1::new();
        let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
        check!(matches!(
            owner.bind_exact_subject(&candidate, &mut grant),
            Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
                "active attachment subject differs"
            )))
        ));
        poisoned(&mut grant)?;
    }
    check!(eq; retained_state(&case.fixture.store)?, before);
    Ok(())
}

#[test]
fn revoked_subject_is_absent_without_discarding_the_original_retained_record() -> TestResult {
    let case = committed("current-revoked")?;
    let inspection = case.fixture.store.inspect_evidence_subject_attachment_v1(
        AttachmentInspectionRequestV1::Revoke {
            prior: case.record.reference(),
        },
    )?;
    let command = next_command(&case.record, "current-revoke", &inspection)?;
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
        .bind_exact_subject(&case.context.subject, &mut grant)?
        .is_none());
    check!(eq; first_record(&case.fixture.store)?, case.record);
    grant.charge_steps(0)?;
    Ok(())
}

#[test]
fn replacement_selects_only_the_active_exact_record_and_its_original_receipt() -> TestResult {
    let case = committed("current-replacement")?;
    let inspection = case.fixture.store.inspect_evidence_subject_attachment_v1(
        AttachmentInspectionRequestV1::Attach {
            target: case.record.command.target.public_id.clone(),
            prior: AttachmentPredecessorV1::After(case.record.reference()),
        },
    )?;
    let command = next_command(&case.record, "current-replacement-successor", &inspection)?;
    let receipt = written(apply(
        &case.fixture.store,
        case.fixture.store.prepare_evidence_subject_attachment_v1(
            command.clone(),
            Preparation::New {
                inspection,
                context: &[],
            },
        )?,
    )?)?;
    let view = case.fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = view.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let bound = owner
        .bind_exact_subject(&case.context.subject, &mut grant)?
        .ok_or("replacement absent")?;
    bound.require_original_grant(&mut grant)?;
    check!(eq; bound.record().command, command);
    check!(eq; bound.receipt(), &receipt);
    check!(ne; bound.record(), &case.record);
    check!(eq; first_record(&case.fixture.store)?, case.record);
    Ok(())
}
