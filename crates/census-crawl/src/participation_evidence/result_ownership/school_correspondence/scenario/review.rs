use super::*;
use census_store::{
    ResultSchoolCorrespondenceInspectionV1, ResultSchoolCorrespondencePreparationV1,
};

pub(super) fn proposal(
    store: &Store,
    input: &setup::Input,
) -> TestResult<ResultSchoolSourceProposalV1> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let provider =
        read_recorded_provider_school(&snapshot, &input.school_packet.reference()?, &mut grant)?
            .ok_or("recorded provider school")?;
    let discovered = discover_current_html_results(&current, &mut grant)?;
    check!(eq; discovered.group_count(), 1);
    check!(eq; discovered.diagnostic_count(), 0);
    let scope = scope()?;
    Ok(discovered.with_group(0, &mut grant, |linked, grant| {
        build_claim(&current, &linked, 0, &provider, scope, grant)
    })?)
}

pub(super) fn command(
    inspection: &ResultSchoolCorrespondenceInspectionV1<'_>,
    source: &ResultSchoolSourceClaimV1,
    operation: &str,
    action: ResultSchoolActionV1,
) -> TestResult<ResultSchoolReviewCommandV1> {
    Ok(ResultSchoolReviewCommandV1 {
        version: 1, policy: text(RESULT_SCHOOL_POLICY_V1)?, store: inspection.store_identity().clone(),
        operation: text(operation)?, input_token: inspection.input_token().clone(),
        evidence_generation: inspection.evidence_generation(), derived_generation: inspection.derived_generation(),
        accepted_head: inspection.public_head().clone(), expected_inventory: inspection.inventory().clone(),
        source: source.clone(), action, actor: ResultSchoolReviewActorV1::LocalOperator, reviewed_at: at()?,
        rationale: text("Explicit protected original unique school-cell correspondence review; exact performance day only, no enrollment or eligibility interval")?,
    })
}

pub(super) fn accept(
    store: &Store,
    proposal: &ResultSchoolSourceProposalV1,
) -> TestResult<(ResultSchoolReviewCommandV1, census_store::Receipt)> {
    let inspection =
        store.inspect_result_school_correspondence_v1(AttachmentReplayGrantV1::new())?;
    let command = command(
        &inspection,
        proposal.claim(),
        "school-correspondence-public-accept",
        ResultSchoolActionV1::Accept {
            prior: ResultSchoolPredecessorV1::First,
        },
    )?;
    let context = proposal.borrowed_context(&mut AttachmentReplayGrantV1::new())?;
    let prepared = store
        .prepare_result_school_correspondence_v1(
            command.clone(),
            ResultSchoolCorrespondencePreparationV1::New {
                inspection,
                context: &context,
            },
        )
        .map_err(|error| format!("prepare checked school review: {error}"))?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    match store
        .apply_result_school_correspondence_v1(prepared, &approved, &digest)
        .map_err(|error| format!("commit checked school review: {error}"))?
    {
        Application::Written(receipt) => Ok((command, receipt)),
        Application::Repeated(_) => Err("new original school review unexpectedly replayed".into()),
    }
}

pub(super) fn observe(store: &Store) -> TestResult<Option<census_domain::model::SchoolId>> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let schools = snapshot.read_current_result_school_correspondences_v1(&current, &mut grant)?;
    check!(!schools.records().is_empty());
    let bytes = grant.usage().encoded_bytes();
    let active = schools.active_records(&mut grant)?.count();
    check!(eq; grant.usage().encoded_bytes(), bytes);
    check!(eq; schools.active_records(&mut grant)?.count(), active);
    check!(eq; grant.usage().encoded_bytes(), bytes);
    let discovered = discover_current_html_results(&current, &mut grant)?;
    check!(eq; discovered.group_count(), 1);
    Ok(discovered.with_group(0, &mut grant, |linked, grant| {
        Ok(
            read::checked_current(&schools, &current, &linked, 0, grant)?
                .map(|checked| checked.school().clone()),
        )
    })?)
}

pub(super) fn record(store: &Store) -> TestResult<ResultSchoolReviewedRecordV1> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let current =
        snapshot.read_current_result_school_correspondences_v1(&attachments, &mut grant)?;
    let record = current.records().first().ok_or("review record absent")?;
    Ok(record.clone())
}

pub(super) fn revoke(
    store: &Store,
    proposal: &ResultSchoolSourceProposalV1,
    previous: &ResultSchoolReviewedRecordV1,
) -> TestResult {
    let inspection =
        store.inspect_result_school_correspondence_v1(AttachmentReplayGrantV1::new())?;
    let command = command(
        &inspection,
        proposal.claim(),
        "school-correspondence-public-revoke",
        ResultSchoolActionV1::Revoke {
            prior: ResultSchoolRecordReferenceV1 {
                operation: previous.command.operation.clone(),
                digest: previous.record_digest.clone(),
            },
        },
    )?;
    let context = proposal.borrowed_context(&mut AttachmentReplayGrantV1::new())?;
    let prepared = store.prepare_result_school_correspondence_v1(
        command,
        ResultSchoolCorrespondencePreparationV1::New {
            inspection,
            context: &context,
        },
    )?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(matches!(
        store.apply_result_school_correspondence_v1(prepared, &approved, &digest)?,
        Application::Written(_)
    ));
    Ok(())
}

pub(super) fn replay(
    store: &Store,
    command: ResultSchoolReviewCommandV1,
) -> TestResult<Application> {
    let prepared = store.prepare_result_school_correspondence_v1(
        command,
        ResultSchoolCorrespondencePreparationV1::ReplayOnly,
    )?;
    check!(prepared.is_replay());
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    Ok(store.apply_result_school_correspondence_v1(prepared, &approved, &digest)?)
}
