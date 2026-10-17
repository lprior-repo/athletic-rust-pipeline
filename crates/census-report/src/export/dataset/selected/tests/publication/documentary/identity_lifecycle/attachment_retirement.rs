use super::*;
use census_domain::evidence_subject_attachment::{
    AttachmentReplayGrantV1, ReviewedReferenceRecordV1,
};
use census_store::{Application, EvidenceSubjectAttachmentPreparationV1, Receipt};

pub(super) struct Retired {
    records: Vec<ReviewedReferenceRecordV1>,
    receipts: Vec<Receipt>,
}

pub(super) fn retire_status(store: &Store) -> TestResult<Retired> {
    let (records, receipts) = retained(store)?;
    check!(eq; records.len(), 3);
    check!(eq; receipts.len(), 3);
    let article = records.first().ok_or("original status A")?;
    check!(eq; article.command.operation.as_str(), "genuine-reviewed-article-a-v1");
    check!(eq; records.get(1).ok_or("original result B")?.command.operation.as_str(), "genuine-reviewed-may15-b-v1");
    check!(eq; records.last().ok_or("result B retirement")?.command.operation.as_str(), "genuine-documentary-controlled-revoke-v1");
    let receipt = super::super::lifecycle::operation::revoke_record(
        store,
        article,
        "controlled-identity-stale-status-a-retirement-v1",
    )?;
    let (after, after_receipts) = retained(store)?;
    check!(eq; after.len(), 4);
    check!(eq; after_receipts.len(), 4);
    check!(eq; after.get(..3), Some(records.as_slice()));
    check!(eq; after_receipts.get(..3), Some(receipts.as_slice()));
    check!(eq; after_receipts.last(), Some(&receipt));
    Ok(Retired {
        records: after,
        receipts: after_receipts,
    })
}

impl Retired {
    pub(super) fn replay_without_reactivation(&self, store: &Store) -> TestResult {
        let before = state(store)?;
        for (record, receipt) in self.records.iter().zip(&self.receipts).take(2) {
            let prepared = store.prepare_evidence_subject_attachment_v1(
                record.command.clone(),
                EvidenceSubjectAttachmentPreparationV1::ReplayOnly,
            )?;
            check!(prepared.is_replay());
            let approval = prepared.approval_bytes().to_vec();
            let digest = prepared.approval_digest().clone();
            check!(eq; store.apply_evidence_subject_attachment_v1(prepared, &approval, &digest)?, Application::Repeated(receipt.clone()));
        }
        check!(eq; state(store)?, before);
        let (records, receipts) = retained(store)?;
        check!(eq; records, self.records);
        check!(eq; receipts, self.receipts);
        let snapshot = store.snapshot();
        let mut grant = AttachmentReplayGrantV1::new();
        let current =
            snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
        for record in self.records.iter().take(2) {
            for subject in record.command.subjects.as_slice() {
                check!(current.bind_exact_subject(subject, &mut grant)?.is_none());
            }
        }
        Ok(())
    }
}

fn retained(store: &Store) -> TestResult<(Vec<ReviewedReferenceRecordV1>, Vec<Receipt>)> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let retained = snapshot.read_evidence_subject_attachment_references_v1(&mut grant)?;
    Ok((retained.records().to_vec(), retained.receipts().to_vec()))
}

fn state(store: &Store) -> TestResult<Value> {
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("retained identity")?;
    Ok(
        json!({"sequence":snapshot.sequence(), "receipts":store.receipt_count()?,
        "evidence_generation":snapshot.evidence_generation(), "derived_generation":snapshot.derived_generation(),
        "tables":snapshot.tables_digest(&Table::ALL)?, "public_head":accepted.head()}),
    )
}
