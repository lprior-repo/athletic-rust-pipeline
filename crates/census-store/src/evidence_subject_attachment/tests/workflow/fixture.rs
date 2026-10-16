use crate::evidence_subject_attachment::{
    AttachmentInspectionRequestV1, EvidenceSubjectAttachmentInspectionV1,
    EvidenceSubjectAttachmentPreparationV1, PreparedEvidenceSubjectAttachmentV1,
    RetainedEvidenceSubjectAttachmentReferencesV1,
};
use crate::public_identity_tests::support::{fixture, subject, PhysicalState, TestResult};
use crate::{Application, IdentityCommitOutcomeV1, PublicIdentityWriterV1, Receipt, Store, Table};
use census_domain::evidence_subject_attachment::*;
use census_domain::model::CanonicalAthlete;
use census_domain::public_identity_history::PublicAthleteId;
use sha2::{Digest, Sha256};

pub(in crate::evidence_subject_attachment) struct Fixture {
    pub directory: tempfile::TempDir,
    pub store: Store,
    pub writer: PublicIdentityWriterV1,
    pub first: IdentityCommitOutcomeV1,
    pub second: CanonicalAthlete,
}

impl Fixture {
    pub fn new() -> TestResult<Self> {
        let (directory, store, writer) = fixture()?;
        let first = subject("attachment-public-workflow-first", "93101")?;
        let second = subject("attachment-public-workflow-second", "93102")?;
        store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
        let first = writer.establish_source_bound(
            &store,
            "attachment-workflow-first-birth",
            &first.id.cast(),
            store.snapshot().revision(),
            &writer.head(&store)?,
        )?;
        Ok(Self {
            directory,
            store,
            writer,
            first,
            second,
        })
    }

    pub fn second_birth(&self) -> TestResult<IdentityCommitOutcomeV1> {
        Ok(self.writer.establish_source_bound(
            &self.store,
            "attachment-workflow-second-birth",
            &self.second.id.cast(),
            self.store.snapshot().revision(),
            &self.writer.head(&self.store)?,
        )?)
    }

    pub fn inspect(&self) -> TestResult<EvidenceSubjectAttachmentInspectionV1<'_>> {
        inspect_first(&self.store, self.first.public_id())
    }
}

pub(super) fn text(value: &str) -> TestResult<AttachmentTextV1> {
    Ok(AttachmentTextV1::parse(value)?)
}

pub(super) fn list<T>(values: Vec<T>) -> TestResult<AttachmentListV1<T>> {
    Ok(AttachmentListV1::new(values)?)
}

pub(super) fn reference(bytes: &[u8]) -> TestResult<AttachmentObjectReferenceV1> {
    Ok(AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(bytes)))?,
        bytes: u64::try_from(bytes.len())?,
    })
}

pub(in crate::evidence_subject_attachment) fn inspect_first<'s>(
    store: &'s Store,
    target: &PublicAthleteId,
) -> TestResult<EvidenceSubjectAttachmentInspectionV1<'s>> {
    Ok(
        store.inspect_evidence_subject_attachment_v1(AttachmentInspectionRequestV1::Attach {
            target: target.clone(),
            prior: AttachmentPredecessorV1::First,
        })?,
    )
}

pub(in crate::evidence_subject_attachment) fn prepare<'s>(
    store: &'s Store,
    inspection: EvidenceSubjectAttachmentInspectionV1<'s>,
    command: RetainedEvidenceSubjectAttachmentCommandV1,
    context: &Context,
) -> TestResult<PreparedEvidenceSubjectAttachmentV1<'s>> {
    Ok(store.prepare_evidence_subject_attachment_v1(
        command,
        EvidenceSubjectAttachmentPreparationV1::New {
            inspection,
            context: &context.borrowed(),
        },
    )?)
}

pub(in crate::evidence_subject_attachment) fn apply(
    store: &Store,
    prepared: PreparedEvidenceSubjectAttachmentV1<'_>,
) -> TestResult<Application> {
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    Ok(store.apply_evidence_subject_attachment_v1(prepared, &approval, &digest)?)
}

pub(in crate::evidence_subject_attachment) fn first_record(
    store: &Store,
) -> TestResult<ReviewedReferenceRecordV1> {
    let view = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let read = view.read_evidence_subject_attachment_references_v1(&mut grant)?;
    Ok(read
        .records()
        .first()
        .ok_or("first attachment record absent")?
        .clone())
}

pub(in crate::evidence_subject_attachment) fn next_command(
    previous: &ReviewedReferenceRecordV1,
    operation: &str,
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
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

pub(super) fn replay(
    store: &Store,
    command: RetainedEvidenceSubjectAttachmentCommandV1,
) -> TestResult<Application> {
    let prepared = store.prepare_evidence_subject_attachment_v1(
        command,
        EvidenceSubjectAttachmentPreparationV1::ReplayOnly,
    )?;
    check!(prepared.is_replay());
    apply(store, prepared)
}

pub(super) fn active_operations(
    read: &RetainedEvidenceSubjectAttachmentReferencesV1<'_>,
) -> TestResult<Vec<String>> {
    let first = read.records().first().ok_or("attachment history absent")?;
    let bindings = read
        .records()
        .iter()
        .map(|record| AttachmentReceiptBindingV1 {
            operation: record.command.operation.clone(),
            record_digest: record.record_digest.clone(),
        })
        .collect::<Vec<_>>();
    let projected = replay_reviewed_references(
        &first.command.store,
        read.records(),
        &bindings,
        read.inventory(),
    )?;
    Ok(projected
        .active_records()
        .map(|record| record.command.operation.as_str().to_owned())
        .collect())
}

pub(super) fn preserved_except_single_attachment(
    before: &PhysicalState,
    after: &PhysicalState,
    receipt: &Receipt,
) -> TestResult {
    let root = Store::journal_key(
        "evidence_subject",
        "attachment/v1/record/00000000000000000001",
    );
    let expected_receipt = serde_json::to_vec(receipt)?;
    let mut root_count = 0usize;
    let mut receipt_count = 0usize;
    let mut preserved = Vec::new();
    for (name, rows) in after {
        let mut retained = Vec::new();
        for (key, value) in rows {
            if name == crate::JOURNAL && key == &root {
                root_count = root_count.checked_add(1).ok_or("root count overflow")?;
                check!(!value.is_empty());
            } else if name == crate::RECEIPTS && key == receipt.operation.as_bytes() {
                receipt_count = receipt_count
                    .checked_add(1)
                    .ok_or("receipt count overflow")?;
                check!(eq; value, &expected_receipt);
            } else {
                retained.push((key.clone(), value.clone()));
            }
        }
        preserved.push((name.clone(), retained));
    }
    check!(eq; root_count, 1);
    check!(eq; receipt_count, 1);
    check!(eq; &preserved, before);
    Ok(())
}

pub(in crate::evidence_subject_attachment) fn written(
    application: Application,
) -> TestResult<Receipt> {
    match application {
        Application::Written(receipt) => Ok(receipt),
        Application::Repeated(receipt) => {
            Err(format!("new operation replayed unexpectedly: {receipt:?}").into())
        }
    }
}

mod context;
pub(in crate::evidence_subject_attachment) use context::{Context, InlineObject};
