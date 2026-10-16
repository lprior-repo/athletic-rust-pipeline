use super::*;
use crate::model::AthleteCandidateId;
use crate::public_identity_history::PublicAthleteId;
use chrono::{DateTime, Utc};

pub(super) fn digest(label: &str) -> TestResult<AttachmentDigestV1> {
    Ok(AttachmentDigestV1::of(label)?)
}
pub(super) fn text(value: &str) -> TestResult<AttachmentTextV1> {
    Ok(AttachmentTextV1::parse(value)?)
}
pub(super) fn store() -> TestResult<StoreIdentity> {
    Ok(StoreIdentity::parse(&"1".repeat(64))?)
}
pub(super) fn list<T>(values: Vec<T>) -> TestResult<AttachmentListV1<T>> {
    Ok(AttachmentListV1::new(values)?)
}
pub(super) fn stamp() -> TestResult<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339("2026-10-16T00:00:00Z")?.with_timezone(&Utc))
}

pub(super) fn command(operation: &str) -> TestResult<RetainedEvidenceSubjectAttachmentCommandV1> {
    let subject = subject("source-row")?;
    let context = context(&subject)?;
    Ok(RetainedEvidenceSubjectAttachmentCommandV1 {
        version: 1, policy: text(EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1)?, store: store()?, operation: text(operation)?,
        expected_public_head: AttachmentPublicHeadV1 { sequence: 1, digest: digest("public head")? },
        expected_evidence_generation: 2, expected_derived_generation: 3,
        expected_attachment_inventory: AttachmentInventoryV1::empty()?,
        target: AttachmentTargetV1 {
            public_id: PublicAthleteId::parse("ath_public_v1_2e4fb039913a48c1fc1c41f2308f4ffc47f0e2b356e2cf14887893066269fa60")?,
            canonical_candidate: AthleteCandidateId::mint("ath", &["controlled target"]),
            accepted_group_digest: digest("accepted group")?,
        },
        subjects: list(vec![subject])?, action: AttachmentActionV1::Attach { prior: AttachmentPredecessorV1::First },
        reviewed_context: context, actor: AttachmentReviewActorV1::LocalOperator,
        reviewed_at: stamp()?, rationale: text("Controlled structural reference fixture; no original fact or accepted person capability")?,
    })
}

pub(super) fn subject(label: &str) -> TestResult<RetainedDocumentSubjectReferenceV1> {
    Ok(RetainedDocumentSubjectReferenceV1 {
        namespace: text("controlled_document_v1")?,
        native_id: Some(text("original-document-id")?),
        capture: digest("capture")?,
        body: digest("body")?,
        interpretation: digest("interpretation")?,
        locator: digest("locator")?,
        subject: digest(label)?,
        context: digest("context")?,
        role: AttachmentSubjectRoleV1::ResultRow,
    })
}

fn context(
    subject: &RetainedDocumentSubjectReferenceV1,
) -> TestResult<ReviewedAttachmentContextV1> {
    let target_subject = digest("target source subject")?;
    let mut facts = vec![
        ReviewedAttachmentFactV1 {
            fact: digest("source fact")?,
            subject: subject.subject.clone(),
            publisher_lineage: digest("source lineage")?,
            disposition: AttachmentFactDispositionV1::SupportsAttachment,
            rationale: text("Explicit retained cross-reference, structural fixture only")?,
        },
        ReviewedAttachmentFactV1 {
            fact: digest("target fact")?,
            subject: target_subject.clone(),
            publisher_lineage: digest("target lineage")?,
            disposition: AttachmentFactDispositionV1::SupportsAttachment,
            rationale: text("Corresponding retained target-side reference")?,
        },
    ];
    facts.sort_by(|left, right| left.fact.cmp(&right.fact));
    let witness = digest("accepted target witness")?;
    let scope = digest("complete scope")?;
    let mut objects = vec![
        subject.capture.clone(),
        subject.body.clone(),
        subject.interpretation.clone(),
        subject.locator.clone(),
        subject.subject.clone(),
        subject.context.clone(),
        target_subject.clone(),
        witness.clone(),
        scope.clone(),
    ];
    for fact in &facts {
        objects.extend([fact.fact.clone(), fact.publisher_lineage.clone()]);
    }
    objects.sort();
    objects.dedup();
    Ok(ReviewedAttachmentContextV1 {
        objects: list(
            objects
                .into_iter()
                .map(|digest| AttachmentObjectReferenceV1 { digest, bytes: 1 })
                .collect(),
        )?,
        facts: list(facts)?,
        corroboration: list(vec![AttachmentCorroborationPairV1 {
            subject_fact: digest("source fact")?,
            accepted_target_fact: digest("target fact")?,
            basis: AttachmentCorroborationBasisV1::ExplicitOriginalCrossReference,
            explanation: text("Explicit controlled review of paired original references")?,
        }])?,
        complete_scope_records: list(vec![scope])?,
        accepted_target_subjects: list(vec![target_subject])?,
        accepted_target_witness: witness,
    })
}

pub(super) fn seal(
    command: RetainedEvidenceSubjectAttachmentCommandV1,
) -> TestResult<ReviewedReferenceRecordV1> {
    let mut record = ReviewedReferenceRecordV1 {
        prepared_digest: command.prepared_digest()?,
        approval_digest: command.approval_digest()?,
        command,
        committed_at: stamp()?,
        record_digest: digest("placeholder")?,
    };
    record.record_digest = record.calculated_record_digest()?;
    Ok(record)
}

pub(super) fn append(
    records: &mut Vec<ReviewedReferenceRecordV1>,
    mut command: RetainedEvidenceSubjectAttachmentCommandV1,
) -> TestResult {
    command.expected_attachment_inventory = inventory(records)?;
    records.push(seal(command)?);
    Ok(())
}

pub(super) fn inventory(
    records: &[ReviewedReferenceRecordV1],
) -> TestResult<AttachmentInventoryV1> {
    Ok(super::super::inventory::inventory(
        records,
        &mut validate::Grant::new(),
    )?)
}

pub(super) fn receipts(records: &[ReviewedReferenceRecordV1]) -> Vec<AttachmentReceiptBindingV1> {
    records
        .iter()
        .map(|record| AttachmentReceiptBindingV1 {
            operation: record.command.operation.clone(),
            record_digest: record.record_digest.clone(),
        })
        .collect()
}

pub(super) fn replay(
    records: &[ReviewedReferenceRecordV1],
) -> TestResult<ReviewedReferenceProjectionV1<'_>> {
    Ok(replay_reviewed_references(
        &store()?,
        records,
        &receipts(records),
        &inventory(records)?,
    )?)
}

pub(super) fn next(
    previous: &ReviewedReferenceRecordV1,
    operation: &str,
) -> TestResult<RetainedEvidenceSubjectAttachmentCommandV1> {
    let mut command = previous.command.clone();
    command.operation = text(operation)?;
    command.action = AttachmentActionV1::Attach {
        prior: AttachmentPredecessorV1::After(previous.reference()),
    };
    Ok(command)
}

pub(super) fn add_subject(
    command: &mut RetainedEvidenceSubjectAttachmentCommandV1,
    extra: RetainedDocumentSubjectReferenceV1,
) -> TestResult {
    let mut subjects = command.subjects.as_slice().to_vec();
    subjects.push(extra.clone());
    subjects.sort_by(|left, right| left.subject.cmp(&right.subject));
    command.subjects = list(subjects)?;
    add_object(command, extra.subject)
}

pub(super) fn add_object(
    command: &mut RetainedEvidenceSubjectAttachmentCommandV1,
    digest: AttachmentDigestV1,
) -> TestResult {
    let mut objects = command.reviewed_context.objects.as_slice().to_vec();
    objects.push(AttachmentObjectReferenceV1 { digest, bytes: 1 });
    objects.sort();
    command.reviewed_context.objects = list(objects)?;
    Ok(())
}

pub(super) fn pair_subject(
    command: &mut RetainedEvidenceSubjectAttachmentCommandV1,
    subject: AttachmentDigestV1,
) -> TestResult {
    let fact = digest("second subject fact")?;
    let mut facts = command.reviewed_context.facts.as_slice().to_vec();
    facts.push(ReviewedAttachmentFactV1 {
        fact: fact.clone(),
        subject,
        publisher_lineage: digest("source lineage")?,
        disposition: AttachmentFactDispositionV1::SupportsAttachment,
        rationale: text("Second reviewed paired endpoint")?,
    });
    facts.sort_by(|left, right| left.fact.cmp(&right.fact));
    command.reviewed_context.facts = list(facts)?;
    let mut pairs = command.reviewed_context.corroboration.as_slice().to_vec();
    pairs.push(AttachmentCorroborationPairV1 {
        subject_fact: fact.clone(),
        accepted_target_fact: digest("target fact")?,
        basis: AttachmentCorroborationBasisV1::ExplicitOriginalCrossReference,
        explanation: text("Second exact source-to-target pair")?,
    });
    command.reviewed_context.corroboration = list(pairs)?;
    add_object(command, fact)
}
