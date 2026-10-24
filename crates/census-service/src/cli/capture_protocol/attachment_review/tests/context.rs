use super::support::{sha256, TestResult};
use census_domain::evidence_subject_attachment::*;
use census_store::EvidenceSubjectAttachmentInspectionV1;
use chrono::DateTime;

pub(super) struct Object {
    pub reference: AttachmentObjectReferenceV1,
    pub bytes: Vec<u8>,
}

pub(super) fn text(value: &str) -> TestResult<AttachmentTextV1> {
    Ok(AttachmentTextV1::parse(value)?)
}

pub(super) fn list<T>(values: Vec<T>) -> TestResult<AttachmentListV1<T>> {
    Ok(AttachmentListV1::new(values)?)
}

pub(super) fn object(bytes: Vec<u8>) -> TestResult<Object> {
    Ok(Object {
        reference: AttachmentObjectReferenceV1 {
            digest: AttachmentDigestV1::parse(&sha256(&bytes))?,
            bytes: u64::try_from(bytes.len())?,
        },
        bytes,
    })
}

pub(super) fn objects(body: &[u8]) -> TestResult<Vec<Object>> {
    [
        "capture",
        "body",
        "interpretation",
        "locator",
        "subject",
        "context",
        "target-subject",
        "source-fact",
        "target-fact",
        "source-lineage",
        "target-lineage",
        "scope",
    ]
    .into_iter()
    .map(|role| {
        let bytes = if role == "body" {
            body.to_vec()
        } else {
            serde_json::to_vec(&serde_json::json!({
                "controlled_synthetic_adapter_fixture": true,
                "role": role,
                "claim": "No genuine-person identity, participation or eligibility acceptance",
            }))?
        };
        object(bytes)
    })
    .collect()
}

pub(super) fn command(
    objects: &[Object],
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
) -> TestResult<RetainedEvidenceSubjectAttachmentCommandV1> {
    let digest = |index: usize| -> TestResult<AttachmentDigestV1> {
        Ok(objects
            .get(index)
            .ok_or("controlled object missing")?
            .reference
            .digest
            .clone())
    };
    let mut inventory: Vec<_> = objects
        .iter()
        .map(|value| value.reference.clone())
        .collect();
    inventory.extend(inspection.issued_objects().cloned());
    inventory.sort();
    inventory.dedup();
    let mut facts = vec![
        ReviewedAttachmentFactV1 {
            fact: digest(7)?,
            subject: digest(4)?,
            publisher_lineage: digest(9)?,
            disposition: AttachmentFactDispositionV1::SupportsAttachment,
            rationale: text(
                "Controlled synthetic source endpoint review, not a real-person claim",
            )?,
        },
        ReviewedAttachmentFactV1 {
            fact: digest(8)?,
            subject: digest(6)?,
            publisher_lineage: digest(10)?,
            disposition: AttachmentFactDispositionV1::SupportsAttachment,
            rationale: text(
                "Controlled synthetic target endpoint review, not a real-person claim",
            )?,
        },
    ];
    facts.sort_by(|left, right| left.fact.cmp(&right.fact));
    Ok(RetainedEvidenceSubjectAttachmentCommandV1 {
        version: 1,
        policy: text(EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1)?,
        store: inspection.store_identity().clone(),
        operation: text("controlled-cli-attachment-review")?,
        expected_public_head: inspection.public_head().clone(),
        expected_evidence_generation: inspection.evidence_generation(),
        expected_derived_generation: inspection.derived_generation(),
        expected_attachment_inventory: inspection.inventory().clone(),
        target: inspection.target().clone(),
        subjects: list(vec![RetainedDocumentSubjectReferenceV1 {
            namespace: text("controlled_cli_attachment_fixture_v1")?,
            native_id: Some(text("controlled-source-subject")?),
            capture: digest(0)?, body: digest(1)?, interpretation: digest(2)?,
            locator: digest(3)?, subject: digest(4)?, context: digest(5)?,
            role: AttachmentSubjectRoleV1::ResultRow,
        }])?,
        action: inspection.action().clone(),
        reviewed_context: ReviewedAttachmentContextV1 {
            objects: list(inventory)?, facts: list(facts)?,
            corroboration: list(vec![AttachmentCorroborationPairV1 {
                subject_fact: digest(7)?, accepted_target_fact: digest(8)?,
                basis: AttachmentCorroborationBasisV1::ExplicitOriginalCrossReference,
                explanation: text("Controlled synthetic paired reference, no genuine athlete join")?,
            }])?,
            complete_scope_records: list(vec![digest(11)?])?,
            accepted_target_subjects: list(vec![digest(6)?])?,
            accepted_target_witness: inspection.accepted_target_witness().clone(),
        },
        actor: AttachmentReviewActorV1::LocalOperator,
        reviewed_at: DateTime::parse_from_rfc3339("2026-10-14T00:00:00Z")?.to_utc(),
        rationale: text("Controlled adapter test approval only; not authenticated human review or real-person acceptance")?,
    })
}
