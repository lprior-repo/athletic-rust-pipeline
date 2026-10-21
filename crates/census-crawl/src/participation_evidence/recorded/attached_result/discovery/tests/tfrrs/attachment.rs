use super::*;
use objects::Objects;

mod context;
mod original;

#[derive(Clone, Copy, Debug)]
pub(super) enum Case {
    Original,
    WrongNamespace,
    ChangedLocator,
    ChangedInterpretation,
}

pub(super) fn apply(store: &Store, input: &setup::Input, case: Case, count: usize) -> TestResult {
    check!(count > 0 && count <= 2);
    for index in 0..count {
        apply_row(store, input, case, index)?;
    }
    Ok(())
}

fn apply_row(store: &Store, input: &setup::Input, case: Case, index: usize) -> TestResult {
    let target = input.targets.get(index).ok_or("checked native target")?;
    let inspection =
        store.inspect_evidence_subject_attachment_v1(AttachmentInspectionRequestV1::Attach {
            target: target.public.clone(),
            prior: AttachmentPredecessorV1::First,
        })?;
    check!(eq; inspection.target().canonical_candidate, target.athlete.id.cast());
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let recorded = read_recorded_tfrrs_list_capture(
        &snapshot,
        &input.context,
        &setup::locators(),
        &mut grant,
    )?
    .ok_or("normal TFRRS retained root")?;
    let checked = recorded.read(&setup::locators())?;
    let mut objects = Objects::default();
    let reviewed = context::build(
        &mut objects,
        &checked,
        index,
        recorded.root(),
        &inspection,
        case,
    )?;
    let mut references = objects.references();
    references.extend(inspection.issued_objects().cloned());
    references.sort();
    references.dedup();
    let command = command(&inspection, index, case, reviewed, references)?;
    let existing = existing(store)?;
    let supplied = objects.borrowed(&existing);
    let prepared = store.prepare_evidence_subject_attachment_v1(
        command,
        EvidenceSubjectAttachmentPreparationV1::New {
            inspection,
            context: &supplied,
        },
    )?;
    check!(!prepared.is_replay());
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(matches!(
        store.apply_evidence_subject_attachment_v1(prepared, &approved, &digest)?,
        Application::Written(_)
    ));
    Ok(())
}

fn command(
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    index: usize,
    case: Case,
    reviewed: context::Review,
    references: Vec<AttachmentObjectReferenceV1>,
) -> TestResult<RetainedEvidenceSubjectAttachmentCommandV1> {
    Ok(RetainedEvidenceSubjectAttachmentCommandV1 {
        version: 1, policy: text(EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1)?,
        store: inspection.store_identity().clone(),
        operation: text(&format!("controlled-tfrrs-attachment-{index}-{case:?}"))?,
        expected_public_head: inspection.public_head().clone(),
        expected_evidence_generation: inspection.evidence_generation(),
        expected_derived_generation: inspection.derived_generation(),
        expected_attachment_inventory: inspection.inventory().clone(),
        target: inspection.target().clone(), subjects: list(vec![reviewed.subject])?,
        action: inspection.action().clone(), actor: AttachmentReviewActorV1::LocalOperator,
        reviewed_at: DateTime::parse_from_rfc3339("2026-10-21T00:00:00Z")?.to_utc(),
        rationale: text("Controlled test decision through the normal attachment protocol; no human production review, school membership, cohort or recruiting eligibility claimed")?,
        reviewed_context: ReviewedAttachmentContextV1 {
            objects: list(references)?, facts: list(reviewed.facts)?,
            corroboration: list(vec![reviewed.pair])?,
            complete_scope_records: list(vec![reviewed.scope])?,
            accepted_target_subjects: list(vec![reviewed.target])?,
            accepted_target_witness: inspection.accepted_target_witness().clone(),
        },
    })
}

fn existing(store: &Store) -> TestResult<Vec<AttachmentObjectReferenceV1>> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let retained = snapshot.read_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(retained.records().len() <= 2);
    Ok(retained
        .records()
        .iter()
        .flat_map(|record| {
            record
                .command
                .reviewed_context
                .objects
                .as_slice()
                .iter()
                .cloned()
        })
        .collect())
}
