use super::*;
use crate::participation_evidence::recorded::read_recorded_independent_participation;
use crate::participation_evidence::ParticipationEvidenceRefV1;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1 as Grant;
use census_store::{DiscoveredAttachmentCaptureV1, RetainedCaptureRootV1};

type StatusRecords = (Vec<RetainedCaptureRootV1>, Vec<ReviewedReferenceRecordV1>);

pub(super) fn matching(store: &Store, target: &PublicAthleteId) -> TestResult<StatusRecords> {
    let snapshot = store.snapshot();
    let mut grant = Grant::new();
    let owner = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let metadata =
        owner.discover_active_subjects_v1(AttachmentSubjectRoleV1::AthleteStatus, &mut grant)?;
    let mut roots = Vec::new();
    let mut records = Vec::new();
    for item in metadata.subjects() {
        let DiscoveredAttachmentCaptureV1::Retained(leaf) = item.capture() else {
            return Err("controlled status must have a retained source".into());
        };
        let reference = ParticipationEvidenceRefV1::parse(&leaf.root.key)?;
        let source = read_recorded_independent_participation(&snapshot, &reference, &mut grant)?
            .ok_or("retained controlled status absent")?;
        check!(eq; source.root(), &leaf.root);
        roots.push(source.root().clone());
        let bound = owner
            .bind_exact_subject(item.subject(), &mut grant)?
            .ok_or("active controlled status absent")?;
        bound.require_original_grant(&mut grant)?;
        check!(eq; &bound.target().public_id, target);
        if source.checked().day().as_str() == "2026-05-15" {
            let record = bound.record();
            if !records.iter().any(|prior: &ReviewedReferenceRecordV1| {
                prior.record_digest == record.record_digest
            }) {
                records.push(record.clone());
            }
        }
    }
    check!(eq; records.len(), 2);
    Ok((roots, records))
}

pub(super) fn require_retained(store: &Store, roots: &[RetainedCaptureRootV1]) -> TestResult {
    let snapshot = store.snapshot();
    let mut grant = Grant::new();
    for root in roots {
        let reference = ParticipationEvidenceRefV1::parse(&root.key)?;
        let source = read_recorded_independent_participation(&snapshot, &reference, &mut grant)?
            .ok_or("withdrawal deleted retained documentary source")?;
        check!(eq; source.root(), root);
    }
    Ok(())
}

pub(super) fn revoke(
    store: &Store,
    original: &ReviewedReferenceRecordV1,
    operation: &str,
) -> TestResult {
    let inspection =
        store.inspect_evidence_subject_attachment_v1(AttachmentInspectionRequestV1::Revoke {
            prior: original.reference(),
        })?;
    let mut command = original.command.clone();
    command.operation = AttachmentTextV1::parse(operation)?;
    command.action = inspection.action().clone();
    command.target = inspection.target().clone();
    command.expected_public_head = inspection.public_head().clone();
    command.expected_attachment_inventory = inspection.inventory().clone();
    command.expected_evidence_generation = inspection.evidence_generation();
    command.expected_derived_generation = inspection.derived_generation();
    command.reviewed_context.accepted_target_witness = inspection.accepted_target_witness().clone();
    command.rationale = AttachmentTextV1::parse(
        "Controlled withdrawal regression, not authenticated production human approval",
    )?;
    let mut objects = command.reviewed_context.objects.as_slice().to_vec();
    objects.extend(inspection.issued_objects().cloned());
    objects.sort();
    objects.dedup();
    command.reviewed_context.objects = AttachmentListV1::new(objects)?;
    let prepared = store.prepare_evidence_subject_attachment_v1(
        command,
        EvidenceSubjectAttachmentPreparationV1::New {
            inspection,
            context: &[],
        },
    )?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(matches!(
        store.apply_evidence_subject_attachment_v1(prepared, &approval, &digest)?,
        Application::Written(_)
    ));
    Ok(())
}
