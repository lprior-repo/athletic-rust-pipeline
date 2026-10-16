use super::*;
use census_domain::evidence_subject_attachment::*;
use census_domain::model::AthleteCandidateId;
use census_domain::public_identity_history::PublicAthleteId;

#[test]
fn actual_entry_encoder_roundtrips_borrowed_record_without_private_wrapper() -> TestResult {
    let record = serialization_record()?;
    let reference = AttachmentObjectReferenceV1 {
        digest: record.record_digest.clone(),
        bytes: 1,
    };
    let introduced = [Material::Inline {
        reference: reference.clone(),
        data: "eA==".into(),
    }];
    let inputs = EntryInputs {
        capture: reference.clone(),
        token: reference,
    };
    let wire = Encoded {
        schema: SCHEMA,
        record: &record,
        introduced: &introduced,
        inputs: &inputs,
    };
    let mut writer_grant = AttachmentReplayGrantV1::new();
    let issued = crate::evidence_subject_attachment::encoding::object(&wire, &mut writer_grant)?;
    let mut read_grant = AttachmentReplayGrantV1::new();
    let decoded = parse(issued.bytes(), &mut read_grant)?;
    check!(eq; decoded.record.get().as_bytes(), census_domain::model::serialized_bytes(&record)?);
    check!(eq; serde_json::to_vec(&decoded)?, issued.bytes());
    check!(eq; decoded.inputs, inputs);
    check!(eq; decoded.introduced.len(), 1);
    check!(eq; read_grant.usage().traversal_steps(), 2);
    check!(eq; read_grant.usage().encoded_bytes(), issued.bytes().len());
    Ok(())
}

fn serialization_record() -> Result<ReviewedReferenceRecordV1, Box<dyn std::error::Error>> {
    let digest = AttachmentDigestV1::parse(&"1".repeat(64))?;
    let at =
        chrono::DateTime::parse_from_rfc3339("2026-10-16T00:00:00Z")?.with_timezone(&chrono::Utc);
    let command = RetainedEvidenceSubjectAttachmentCommandV1 {
        version: 1,
        policy: AttachmentTextV1::parse(EVIDENCE_SUBJECT_ATTACHMENT_POLICY_V1)?,
        store: StoreIdentity::parse(&"2".repeat(64))?,
        operation: AttachmentTextV1::parse("serialization-only \"é\\雪\"")?,
        expected_public_head: AttachmentPublicHeadV1 {
            sequence: 1,
            digest: digest.clone(),
        },
        expected_evidence_generation: u64::MAX,
        expected_derived_generation: 0,
        expected_attachment_inventory: AttachmentInventoryV1 {
            version: 1,
            records: 0,
            digest: digest.clone(),
        },
        target: AttachmentTargetV1 {
            public_id: PublicAthleteId::parse(&format!("ath_public_v1_{}", "3".repeat(64)))?,
            canonical_candidate: AthleteCandidateId::mint("ath", &["serialization only"]),
            accepted_group_digest: digest.clone(),
        },
        subjects: AttachmentListV1::new(Vec::new())?,
        action: AttachmentActionV1::Attach {
            prior: AttachmentPredecessorV1::First,
        },
        reviewed_context: ReviewedAttachmentContextV1 {
            objects: AttachmentListV1::new(Vec::new())?,
            facts: AttachmentListV1::new(Vec::new())?,
            corroboration: AttachmentListV1::new(Vec::new())?,
            complete_scope_records: AttachmentListV1::new(Vec::new())?,
            accepted_target_subjects: AttachmentListV1::new(Vec::new())?,
            accepted_target_witness: digest.clone(),
        },
        actor: AttachmentReviewActorV1::LocalOperator,
        reviewed_at: at,
        rationale: AttachmentTextV1::parse(
            "Serialization fixture only; no domain admission or person acceptance",
        )?,
    };
    Ok(ReviewedReferenceRecordV1 {
        command,
        prepared_digest: digest.clone(),
        approval_digest: digest.clone(),
        committed_at: at,
        record_digest: digest,
    })
}
