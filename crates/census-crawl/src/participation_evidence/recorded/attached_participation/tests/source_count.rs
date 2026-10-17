use super::*;
use census_domain::evidence_subject_attachment::*;
use census_domain::public_identity_history::PublicAthleteId;
use census_store::{
    Application, AttachmentInspectionRequestV1, EvidenceSubjectAttachmentInspectionV1,
    EvidenceSubjectAttachmentPreparationV1, Store, Table,
};
use sha2::{Digest, Sha256};
use std::path::Path;
mod context;
mod material;
mod observe;
mod preserve;

pub(in crate::participation_evidence::recorded) fn verify_source_count(
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
    let schedule = material::schedule(&store)?;
    let mut references = std::collections::BTreeSet::new();
    for index in 1..=16 {
        let packet = material::packet(index, &schedule)?;
        check!(references.insert(packet.reference()?.as_str().to_owned()));
        let recording = crate::Recording::new();
        crate::participation_evidence::recorded::record_independent_participation_packet(
            &store, &recording, &packet,
        )?;
        recording
            .drain()
            .prepare(&format!("synthetic-count-intake-{index}"))?
            .apply(&store)?;
        let inspection = store.inspect_evidence_subject_attachment_v1(
            AttachmentInspectionRequestV1::Attach {
                target: target.clone(),
                prior: AttachmentPredecessorV1::First,
            },
        )?;
        let context =
            context::Context::synthetic(&format!("synthetic-count-{index}"), &store, &packet)?;
        let command = context.command(&format!("synthetic-count-attach-{index}"), &inspection)?;
        let objects = context.borrowed();
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
        if index >= 15 {
            let tables = store.snapshot().tables_digest(&Table::ALL)?;
            let receipts = store.receipt_count()?;
            let current = observe::current(&store, index + 1)?;
            let frozen = observe::frozen(&store, index + 1)?;
            check!(eq; current, frozen);
            if index == 15 {
                check!(references
                    .iter()
                    .all(|reference| current.contains(reference)));
            }
            check!(eq; store.snapshot().tables_digest(&Table::ALL)?, tables);
            check!(eq; store.receipt_count()?, receipts);
        }
    }
    check!(eq; references.len(), 16);
    drop(writer);
    drop(store);
    directory.close()?;
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

fn text(value: &str) -> TestResult<AttachmentTextV1> {
    Ok(AttachmentTextV1::parse(value)?)
}
fn list<T>(value: Vec<T>) -> TestResult<AttachmentListV1<T>> {
    Ok(AttachmentListV1::new(value)?)
}
fn reference(value: &[u8]) -> TestResult<AttachmentObjectReferenceV1> {
    Ok(AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(value)))?,
        bytes: u64::try_from(value.len())?,
    })
}
