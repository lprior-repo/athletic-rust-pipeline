use super::documentary_review::AcceptedSource;
use super::source::{InstalledSource, TestResult};
use census_domain::evidence_subject_attachment::{
    AttachmentListV1, AttachmentReplayGrantV1, AttachmentTextV1,
};
use census_store::{
    Application, AttachmentInspectionRequestV1, EvidenceSubjectAttachmentPreparationV1, Receipt,
};

pub(super) fn revoke(
    installed: &InstalledSource,
    accepted: &AcceptedSource,
) -> TestResult<Receipt> {
    let store = &installed.store;
    let inspection =
        store.inspect_evidence_subject_attachment_v1(AttachmentInspectionRequestV1::Revoke {
            prior: accepted.reference.clone(),
        })?;
    let mut command = accepted.command.clone();
    command.operation = AttachmentTextV1::parse("controlled-documentary-revoke-v1")?;
    command.action = inspection.action().clone();
    command.target = inspection.target().clone();
    command.expected_public_head = inspection.public_head().clone();
    command.expected_attachment_inventory = inspection.inventory().clone();
    command.expected_evidence_generation = inspection.evidence_generation();
    command.expected_derived_generation = inspection.derived_generation();
    command.reviewed_context.accepted_target_witness = inspection.accepted_target_witness().clone();
    command.rationale = AttachmentTextV1::parse(
        "Controlled regression revocation of retained evidence; not authenticated production human approval",
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
    if prepared.is_replay() {
        return Err("revocation preparation returned replay".into());
    }
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    match store.apply_evidence_subject_attachment_v1(prepared, &approval, &digest)? {
        Application::Written(receipt) => Ok(receipt),
        Application::Repeated(_) => {
            Err("controlled documentary revocation unexpectedly replayed".into())
        }
    }
}

pub(super) fn replay_without_reactivation(
    installed: &InstalledSource,
    accepted: &AcceptedSource,
) -> TestResult {
    let store = &installed.store;
    let before_snapshot = store.snapshot();
    let before = (
        before_snapshot.evidence_generation(),
        before_snapshot.derived_generation(),
        store.receipt_count()?,
    );
    let prepared = store.prepare_evidence_subject_attachment_v1(
        accepted.command.clone(),
        EvidenceSubjectAttachmentPreparationV1::ReplayOnly,
    )?;
    if !prepared.is_replay() {
        return Err("replay preparation returned new application".into());
    }
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    match store.apply_evidence_subject_attachment_v1(prepared, &approval, &digest)? {
        Application::Repeated(receipt) => {
            if receipt != accepted.receipt {
                return Err("replay receipt mismatch".into());
            }
        }
        Application::Written(_) => {
            return Err("controlled documentary replay unexpectedly wrote".into());
        }
    }
    let after_snapshot = store.snapshot();
    let after = (
        after_snapshot.evidence_generation(),
        after_snapshot.derived_generation(),
        store.receipt_count()?,
    );
    if before != after {
        return Err("replay changed store state".into());
    }
    let mut grant = AttachmentReplayGrantV1::new();
    let current =
        after_snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    for subject in accepted.command.subjects.as_slice() {
        if current.bind_exact_subject(subject, &mut grant)?.is_some() {
            return Err("revoked documentary subject reactivated".into());
        }
    }
    Ok(())
}
