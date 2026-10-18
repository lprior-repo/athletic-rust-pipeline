use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1 as Grant;
use census_domain::evidence_subject_attachment::*;
use census_domain::public_identity_history::PublicAthleteId;
use census_store::{
    Application, AttachmentInspectionRequestV1, EvidenceSubjectAttachmentPreparationV1, Store,
    Table,
};
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod material;
mod observe;
use super::source_count::preserve;
mod withdraw;

pub(in crate::participation_evidence::recorded) fn verify_date_scope(
    closed_root: &Path,
) -> TestResult {
    let before = preserve::files(closed_root)?;
    let result = scenario(closed_root);
    check!(eq; preserve::files(closed_root)?, before);
    result
}

fn scenario(closed_root: &Path) -> TestResult {
    let directory = tempfile::tempdir()?;
    let backup = directory.path().join("backup");
    let root = directory.path().join("private-store");
    Store::backup(closed_root, &backup)?;
    Store::restore(&backup, &root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    let target = target(&store)?;

    let packet_one = material::packet(1)?;
    let packet_two = material::packet(2)?;

    attach_packet(&store, &target, &packet_one, "date-scope-intake-one")?;
    attach_packet(&store, &target, &packet_two, "date-scope-intake-two")?;

    observe::current(&store, &target, &packet_one, &packet_two)?;

    drop(writer);
    drop(store);
    directory.close()?;
    Ok(())
}

fn attach_packet(
    store: &Store,
    target: &PublicAthleteId,
    packet: &crate::participation_evidence::IndependentParticipationPacketV1,
    operation: &str,
) -> TestResult {
    let recording = crate::Recording::new();
    crate::participation_evidence::recorded::record_independent_participation_packet(
        store, &recording, packet,
    )?;
    crate::recording::prepare_recorded(recording.drain(), operation)?.apply(store)?;
    let inspection =
        store.inspect_evidence_subject_attachment_v1(AttachmentInspectionRequestV1::Attach {
            target: target.clone(),
            prior: AttachmentPredecessorV1::First,
        })?;
    let context = super::source_count::context::Context::synthetic(operation, store, packet)?;
    let command = context.command(operation, &inspection)?;
    let objects = context.borrowed(store)?;
    let prepared = store.prepare_evidence_subject_attachment_v1(
        command,
        EvidenceSubjectAttachmentPreparationV1::New {
            inspection,
            context: &objects,
        },
    )?;
    check!(!prepared.is_replay());
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(matches!(
        store.apply_evidence_subject_attachment_v1(prepared, &approval, &digest)?,
        Application::Written(_)
    ));
    Ok(())
}

fn target(store: &Store) -> TestResult<PublicAthleteId> {
    let snapshot = store.snapshot();
    let mut grant = Grant::new();
    let owner = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let metadata =
        owner.discover_active_subjects_v1(AttachmentSubjectRoleV1::AthleteStatus, &mut grant)?;
    let [item] = metadata.subjects() else {
        return Err("one genuine status subject".into());
    };
    let bound = owner
        .bind_exact_subject(item.subject(), &mut grant)?
        .ok_or("active genuine status")?;
    bound.require_original_grant(&mut grant)?;
    Ok(bound.target().public_id.clone())
}
