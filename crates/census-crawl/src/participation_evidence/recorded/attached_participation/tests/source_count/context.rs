use super::*;
use census_store::AttachmentContextObjectV1;
use chrono::{DateTime, Utc};

pub(in super::super) struct InlineObject {
    pub reference: AttachmentObjectReferenceV1,
    pub bytes: Vec<u8>,
}

impl InlineObject {
    pub fn synthetic(label: &str) -> TestResult<Self> {
        let bytes = serde_json::to_vec(&serde_json::json!({
            "synthetic_test_review": true,
            "role": label,
            "claim": "Controlled reference fixture; no genuine-person acceptance",
        }))?;
        Ok(Self {
            reference: reference(&bytes)?,
            bytes,
        })
    }

    pub fn borrowed(&self) -> AttachmentContextObjectV1<'_> {
        AttachmentContextObjectV1::Inline {
            reference: &self.reference,
            bytes: &self.bytes,
        }
    }
}

pub(in super::super) struct Context {
    pub objects: Vec<InlineObject>,
    pub subject: RetainedDocumentSubjectReferenceV1,
    pub reviewed: ReviewedAttachmentContextV1,
    pub leaves: Vec<census_store::RetainedCaptureLeafV1>,
}

impl Context {
    pub fn synthetic(
        label: &str,
        store: &Store,
        packet: &IndependentParticipationPacketV1,
    ) -> TestResult<Self> {
        let (leaves, original) = material::read(store, packet)?;
        let checked = original.checked();
        let document = checked.subject();
        let proof = checked.status_proof();
        let roles = [
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
        ];
        let mut objects = roles
            .iter()
            .map(|role| InlineObject::synthetic(&format!("{label}/{role}")))
            .collect::<TestResult<Vec<_>>>()?;
        let exact = [
            format!(
                "mhsaa_subject_dcc_event_day_v1\n{}\n{}",
                proof.capture(),
                proof.locator()
            )
            .into_bytes(),
            document.locator().as_bytes().to_vec(),
            format!(
                "document_subject_v1\n{}\n{}\n{}\n{}\n{}",
                document.document_namespace(),
                document.native_document_id(),
                document.document_url(),
                document.body_sha256(),
                document.locator()
            )
            .into_bytes(),
        ];
        for (index, bytes) in (2..=4).zip(exact) {
            *objects
                .get_mut(index)
                .ok_or("synthetic exact object slot")? = InlineObject {
                reference: reference(&bytes)?,
                bytes,
            };
        }
        let digest = |index: usize| -> TestResult<AttachmentDigestV1> {
            Ok(objects
                .get(index)
                .ok_or("synthetic context object missing")?
                .reference
                .digest
                .clone())
        };
        let subject = RetainedDocumentSubjectReferenceV1 {
            namespace: text(document.document_namespace())?,
            native_id: Some(text(document.native_document_id())?),
            capture: leaves
                .first()
                .ok_or("status capture leaf")?
                .object
                .digest
                .clone(),
            body: leaves
                .get(1)
                .ok_or("status body leaf")?
                .object
                .digest
                .clone(),
            interpretation: digest(2)?,
            locator: digest(3)?,
            subject: digest(4)?,
            context: digest(5)?,
            role: AttachmentSubjectRoleV1::AthleteStatus,
        };
        let mut facts = vec![
            ReviewedAttachmentFactV1 {
                fact: digest(7)?,
                subject: digest(4)?,
                publisher_lineage: digest(9)?,
                disposition: AttachmentFactDispositionV1::SupportsAttachment,
                rationale: text("Synthetic controlled source endpoint review")?,
            },
            ReviewedAttachmentFactV1 {
                fact: digest(8)?,
                subject: digest(6)?,
                publisher_lineage: digest(10)?,
                disposition: AttachmentFactDispositionV1::SupportsAttachment,
                rationale: text("Synthetic controlled target endpoint review")?,
            },
        ];
        facts.sort_by(|left, right| left.fact.cmp(&right.fact));
        let reviewed = ReviewedAttachmentContextV1 {
            objects: list(Vec::new())?,
            facts: list(facts)?,
            corroboration: list(vec![AttachmentCorroborationPairV1 {
                subject_fact: digest(7)?,
                accepted_target_fact: digest(8)?,
                basis: AttachmentCorroborationBasisV1::ExplicitOriginalCrossReference,
                explanation: text("Synthetic paired-reference decision; no actual person claim")?,
            }])?,
            complete_scope_records: list(vec![digest(11)?])?,
            accepted_target_subjects: list(vec![digest(6)?])?,
            accepted_target_witness: digest(0)?,
        };
        drop(objects.drain(..2));
        Ok(Self {
            leaves,
            objects,
            subject,
            reviewed,
        })
    }

    pub fn command(
        &self,
        operation: &str,
        inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    ) -> TestResult<RetainedEvidenceSubjectAttachmentCommandV1> {
        let mut reviewed = self.reviewed.clone();
        let mut objects = self
            .objects
            .iter()
            .map(|object| object.reference.clone())
            .collect::<Vec<_>>();
        objects.extend(self.leaves.iter().map(|leaf| leaf.object.clone()));
        objects.extend(inspection.issued_objects().cloned());
        objects.sort();
        objects.dedup();
        reviewed.objects = list(objects)?;
        reviewed.accepted_target_witness = inspection.accepted_target_witness().clone();
        Ok(RetainedEvidenceSubjectAttachmentCommandV1 {
            version: 1, policy: text(EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1)?,
            store: inspection.store_identity().clone(), operation: text(operation)?,
            expected_public_head: inspection.public_head().clone(),
            expected_evidence_generation: inspection.evidence_generation(),
            expected_derived_generation: inspection.derived_generation(),
            expected_attachment_inventory: inspection.inventory().clone(),
            target: inspection.target().clone(), subjects: list(vec![self.subject.clone()])?,
            action: inspection.action().clone(), reviewed_context: reviewed,
            actor: AttachmentReviewActorV1::LocalOperator,
            reviewed_at: DateTime::parse_from_rfc3339("2026-10-16T00:11:00Z")?.with_timezone(&Utc),
            rationale: text("Controlled synthetic workflow acceptance; approval bytes are a test decision, not authenticated human review")?,
        })
    }

    pub fn borrowed(&self) -> Vec<AttachmentContextObjectV1<'_>> {
        self.objects
            .iter()
            .map(InlineObject::borrowed)
            .chain(self.leaves.iter().map(AttachmentContextObjectV1::Retained))
            .collect()
    }
}
