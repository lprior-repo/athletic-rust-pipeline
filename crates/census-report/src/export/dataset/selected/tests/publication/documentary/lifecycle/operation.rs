use super::TestResult;
use census_domain::evidence_subject_attachment::{
    AttachmentListV1, AttachmentReplayGrantV1, AttachmentTextV1, ReviewedReferenceRecordV1,
};
use census_store::{
    Application, AttachmentInspectionRequestV1, EvidenceSubjectAttachmentPreparationV1,
    PreparedEvidenceSubjectAttachmentV1, Receipt, Store, Table,
};

pub(in super::super) struct Revoked {
    records: Vec<ReviewedReferenceRecordV1>,
    receipts: Vec<Receipt>,
    revocation: Receipt,
}

pub(in super::super) fn revoke(store: &Store) -> TestResult<Revoked> {
    let (records, receipts) = retained(store)?;
    check!(eq; records.len(), 2);
    check!(eq; receipts.len(), 2);
    let original = records.last().ok_or("genuine B attachment")?;
    check!(eq; original.command.operation.as_str(), "genuine-reviewed-may15-b-v1");
    let before = state(store)?;
    let inspection =
        store.inspect_evidence_subject_attachment_v1(AttachmentInspectionRequestV1::Revoke {
            prior: original.reference(),
        })?;
    let mut command = original.command.clone();
    command.operation = AttachmentTextV1::parse("genuine-documentary-controlled-revoke-v1")?;
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
    check!(!prepared.is_replay());
    let Application::Written(revocation) = apply(store, prepared)? else {
        return Err("new genuine attachment revocation must write".into());
    };
    check!(eq; state(store)?, before);
    Ok(Revoked {
        records,
        receipts,
        revocation,
    })
}

pub(super) fn replay_and_require_revoked(store: &Store, original: &Revoked) -> TestResult {
    let before = (
        store.snapshot().sequence(),
        store.receipt_count()?,
        state(store)?,
    );
    for (record, receipt) in original.records.iter().zip(&original.receipts) {
        let prepared = store.prepare_evidence_subject_attachment_v1(
            record.command.clone(),
            EvidenceSubjectAttachmentPreparationV1::ReplayOnly,
        )?;
        check!(prepared.is_replay());
        check!(eq; apply(store, prepared)?, Application::Repeated(receipt.clone()));
    }
    check!(eq; (store.snapshot().sequence(), store.receipt_count()?, state(store)?), before);
    let (records, receipts) = retained(store)?;
    check!(eq; records.len(), 3);
    check!(eq; receipts.len(), 3);
    check!(eq; records.get(..2), Some(original.records.as_slice()));
    check!(eq; receipts.get(..2), Some(original.receipts.as_slice()));
    check!(eq; receipts.last(), Some(&original.revocation));
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let article = original.records.first().ok_or("retained article A")?;
    check!(eq; article.command.operation.as_str(), "genuine-reviewed-article-a-v1");
    for subject in article.command.subjects.as_slice() {
        let bound = current
            .bind_exact_subject(subject, &mut grant)?
            .ok_or("unrevoked article A")?;
        bound.require_original_grant(&mut grant)?;
        check!(eq; bound.record(), article);
        check!(eq; Some(bound.receipt()), original.receipts.first());
    }
    let results = original.records.last().ok_or("revoked results B")?;
    for subject in results.command.subjects.as_slice() {
        check!(current.bind_exact_subject(subject, &mut grant)?.is_none());
    }
    let plan = census_crawl::participation_evidence::recorded::discover_current_html_results(
        &current, &mut grant,
    )?;
    check!(eq; plan.group_count(), 0);
    check!(eq; plan.diagnostic_count(), 0);
    Ok(())
}

fn apply(
    store: &Store,
    prepared: PreparedEvidenceSubjectAttachmentV1<'_>,
) -> TestResult<Application> {
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    Ok(store.apply_evidence_subject_attachment_v1(prepared, &approval, &digest)?)
}

fn retained(store: &Store) -> TestResult<(Vec<ReviewedReferenceRecordV1>, Vec<Receipt>)> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let retained = snapshot.read_evidence_subject_attachment_references_v1(&mut grant)?;
    Ok((retained.records().to_vec(), retained.receipts().to_vec()))
}

fn state(store: &Store) -> TestResult<(u64, u64, String, String)> {
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("accepted identity")?;
    Ok((
        snapshot.evidence_generation(),
        snapshot.derived_generation(),
        snapshot.tables_digest(&Table::ALL)?,
        serde_json::to_string(accepted.head())?,
    ))
}
