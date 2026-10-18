use super::*;
use census_store::{
    AttachmentInspectionRequestV1, EvidenceSubjectAttachmentInspectionV1,
    EvidenceSubjectAttachmentPreparationV1, ResultSchoolCorrespondenceStoreError, StoreError,
};

#[test]
fn withdrawing_result_authority_between_school_prepare_and_apply_refuses_without_mutation(
) -> TestResult {
    let (_directory, store, input) =
        setup::install().map_err(|error| format!("install original sources: {error}"))?;
    attachment::apply(&store, &input)
        .map_err(|error| format!("review source attachments: {error}"))?;
    setup::enroll(&store).map_err(|error| format!("enroll correspondence review: {error}"))?;
    let proposal = review::proposal(&store, &input)
        .map_err(|error| format!("build original-school proposal: {error}"))?;
    review::accept(&store, &proposal).map_err(|error| format!("accept school review: {error}"))?;
    let prepared = refusals::prepare_stale(&store, &proposal)
        .map_err(|error| format!("prepare before subject withdrawal: {error}"))?;
    withdraw_person_attachment(&store, &proposal)
        .map_err(|error| format!("withdraw supporting result authority: {error}"))?;
    let originals = setup::originals(&store, &input)?;
    let after_withdrawal = setup::state(&store)?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    match store.apply_result_school_correspondence_v1(prepared, &approved, &digest) {
        Err(StoreError::ResultSchoolCorrespondence(
            ResultSchoolCorrespondenceStoreError::InputMoved,
        )) => {}
        Err(error) => return Err(format!("wrong withdrawal fence refusal: {error}").into()),
        Ok(_) => {
            return Err("school review accepted after its result authority was withdrawn".into())
        }
    }
    check!(eq; setup::state(&store)?, after_withdrawal);
    check!(eq; setup::originals(&store, &input)?, originals);
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(current
        .discover_active_result_subjects_v1(&mut grant)?
        .subjects()
        .is_empty());
    Ok(())
}

fn withdraw_person_attachment(
    store: &Store,
    proposal: &ResultSchoolSourceProposalV1,
) -> TestResult {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let active = attachments.discover_active_result_subjects_v1(&mut grant)?;
    let original = active
        .subjects()
        .iter()
        .map(|subject| subject.record())
        .find(|record| {
            record.command.operation.as_str()
                == proposal.claim().person_attachment.operation.as_str()
        })
        .ok_or("original person attachment record absent")?;
    let inspection =
        store.inspect_evidence_subject_attachment_v1(AttachmentInspectionRequestV1::Revoke {
            prior: proposal.claim().person_attachment.clone(),
        })?;
    let command = build_revoke_command(
        original,
        &inspection,
        "school-correspondence-stale-person-withdrawal",
    )?;
    let prepared = store.prepare_evidence_subject_attachment_v1(
        command,
        EvidenceSubjectAttachmentPreparationV1::New {
            inspection,
            context: &[],
        },
    )?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    let application = store.apply_evidence_subject_attachment_v1(prepared, &approval, &digest)?;
    match application {
        Application::Written(_) => Ok(()),
        Application::Repeated(_) => Err("new withdrawal unexpectedly replayed".into()),
    }
}

fn build_revoke_command(
    previous: &ReviewedReferenceRecordV1,
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    operation: &str,
) -> TestResult<RetainedEvidenceSubjectAttachmentCommandV1> {
    let mut command = previous.command.clone();
    command.operation = text(operation)?;
    command.action = inspection.action().clone();
    command.target = inspection.target().clone();
    command.expected_public_head = inspection.public_head().clone();
    command.expected_attachment_inventory = inspection.inventory().clone();
    command.expected_evidence_generation = inspection.evidence_generation();
    command.expected_derived_generation = inspection.derived_generation();
    command.reviewed_context.accepted_target_witness = inspection.accepted_target_witness().clone();
    let mut objects = command.reviewed_context.objects.as_slice().to_vec();
    objects.extend(inspection.issued_objects().cloned());
    objects.sort();
    objects.dedup();
    command.reviewed_context.objects = list(objects)?;
    Ok(command)
}
