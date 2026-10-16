use super::super::RetainedCaptureRootSchemaV1 as Schema;
use super::*;
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::{Store, StoreError};
use base64::{engine::general_purpose::STANDARD, Engine};
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, EvidenceSubjectAttachmentError,
};
use sha2::{Digest, Sha256};

fn reference(raw: &[u8]) -> TestResult<AttachmentObjectReferenceV1> {
    Ok(AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(raw)))?,
        bytes: u64::try_from(raw.len())?,
    })
}

fn capture(body: &[u8]) -> TestResult<serde_json::Value> {
    Ok(serde_json::json!({
        "requested_url":"https://example.test/source", "final_url":"https://example.test/source",
        "status":200, "acquired_at":"2026-10-16T02:00:00Z", "body_bytes":body.len(),
        "body_sha256":reference(body)?.digest.as_str(), "body_base64":STANDARD.encode(body),
        "provenance":{"request_method":"GET","request_headers":[],"response_header_bytes":[],
            "content_type":"text/html; charset=utf-8","body_encoding":"identity","started_at":"2026-10-16T02:00:00Z"}
    }))
}

fn install(
    store: &Store,
    schema: Schema,
    raw: &str,
    slot: RetainedCaptureSlotV1,
    object: AttachmentObjectReferenceV1,
) -> TestResult<RetainedCaptureLeafV1> {
    let key = "a".repeat(64);
    let envelope = format!(r#"{{"key":"{key}","at":"2026-10-16T02:00:00Z","payload":{raw}}}"#);
    store.journal.insert(
        Store::journal_key(schema.phase(), &key),
        envelope.as_bytes(),
    )?;
    Ok(RetainedCaptureLeafV1 {
        version: 1,
        root: RetainedCaptureRootV1 {
            schema,
            key,
            payload: reference(raw.as_bytes())?,
        },
        slot,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object,
    })
}

#[test]
fn every_fixed_root_slot_verifies_original_body_and_exact_raw_capture() -> TestResult {
    let body = b"original controlled bytes";
    let capture = capture(body)?;
    let capture_raw = serde_json::to_vec(&capture)?;
    let participation = serde_json::json!({"version":1,"status":capture,"schedule":capture});
    let provider = serde_json::json!({"version":1,"interpretation":"milesplit_profile_linked_competition_target_v1","profile":capture,"linked_results":capture});
    for (schema, slot, payload, body_steps) in [
        (
            Schema::ParticipationPacketV1,
            RetainedCaptureSlotV1::ParticipationStatus,
            &participation,
            64,
        ),
        (
            Schema::ParticipationPacketV1,
            RetainedCaptureSlotV1::ParticipationSchedule,
            &participation,
            64,
        ),
        (
            Schema::ProviderTargetPacketV1,
            RetainedCaptureSlotV1::ProviderProfile,
            &provider,
            64,
        ),
        (
            Schema::ProviderTargetPacketV1,
            RetainedCaptureSlotV1::ProviderLinkedResults,
            &provider,
            64,
        ),
        (
            Schema::HtmlResultCaptureV1,
            RetainedCaptureSlotV1::SelfCapture,
            &capture,
            56,
        ),
    ] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let mut leaf = install(
            &store,
            schema,
            &serde_json::to_string(payload)?,
            slot,
            reference(body)?,
        )?;
        let before = retained_state(&store)?;
        let mut grant = AttachmentReplayGrantV1::new();
        verify(&store.snapshot(), &leaf, &mut grant)?;
        check!(eq; grant.usage().traversal_steps(), body_steps);
        check!(eq; grant.usage().encoded_bytes(), 0);
        leaf.kind = RetainedCaptureLeafKindV1::CaptureJson;
        leaf.object = reference(&capture_raw)?;
        verify(&store.snapshot(), &leaf, &mut grant)?;
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}

#[test]
fn whole_root_rejects_duplicate_escaped_duplicate_unknown_and_late_other_capture() -> TestResult {
    let capture = serde_json::to_string(&capture(b"context")?)?;
    let bad_capture = capture.replacen("{", "{\"unexpected\":true,", 1);
    for raw in [
        format!(r#"{{"version":1,"status":{capture},"schedule":{capture},"version":1}}"#),
        format!(r#"{{"version":1,"status":{capture},"schedule":{capture},"\u0076ersion":1}}"#),
        format!(r#"{{"version":1,"status":{capture},"schedule":{capture},"extra":null}}"#),
        format!(r#"{{"version":1,"status":{capture},"schedule":{bad_capture}}}"#),
        format!(r#"[1,{capture},{capture}]"#),
    ] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let leaf = install(
            &store,
            Schema::ParticipationPacketV1,
            &raw,
            RetainedCaptureSlotV1::ParticipationStatus,
            reference(b"context")?,
        )?;
        let before = retained_state(&store)?;
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            verify(&store.snapshot(), &leaf, &mut grant),
            Err(StoreError::Decode { .. }
                | StoreError::EvidenceSubjectAttachment(Error::Corrupt(
                    "external schema requires a JSON object"
                )))
        ));
        check!(matches!(
            grant.ensure_available(),
            Err(EvidenceSubjectAttachmentError::Poisoned)
        ));
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}

#[test]
fn body_bound_accepts_exact_and_refuses_one_over_without_a_leaf() -> TestResult {
    for length in [512 * 1024, 512 * 1024 + 1] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let body = vec![b'x'; length];
        let raw = serde_json::to_string(&capture(&body)?)?;
        let leaf = install(
            &store,
            Schema::HtmlResultCaptureV1,
            &raw,
            RetainedCaptureSlotV1::SelfCapture,
            reference(&body)?,
        )?;
        let before = retained_state(&store)?;
        let mut grant = AttachmentReplayGrantV1::new();
        let result = verify(&store.snapshot(), &leaf, &mut grant);
        if length == 512 * 1024 {
            result?;
        } else {
            check!(matches!(
                result,
                Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
                    "external capture body bounds"
                )))
            ));
        }
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}

#[test]
fn schema_slot_root_reference_body_reference_and_encoded_representation_are_exact() -> TestResult {
    let body = b"context";
    let good = serde_json::to_string(&capture(body)?)?;
    for case in 0..7 {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let raw = match case {
            4 => good.replace("Y29udGV4dA==", "\\u005929udGV4dA=="),
            5 => good.replace("Y29udGV4dA==", "============"),
            6 => good.replace("Y29udGV4dA==", "Y29udGV4dQ=="),
            _ => good.clone(),
        };
        let mut leaf = install(
            &store,
            Schema::HtmlResultCaptureV1,
            &raw,
            RetainedCaptureSlotV1::SelfCapture,
            reference(body)?,
        )?;
        match case {
            0 => leaf.version = 2,
            1 => leaf.slot = RetainedCaptureSlotV1::ProviderProfile,
            2 => leaf.root.payload = reference(b"different root")?,
            3 => leaf.object = reference(b"different body")?,
            _ => {}
        }
        let before = retained_state(&store)?;
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            verify(&store.snapshot(), &leaf, &mut grant),
            Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(_)))
        ));
        check!(matches!(
            grant.ensure_available(),
            Err(EvidenceSubjectAttachmentError::Poisoned)
        ));
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}

#[test]
fn selected_capture_and_provenance_are_maps_with_unique_known_fields() -> TestResult {
    let original = capture(b"context")?;
    let mut array_provenance = original.clone();
    array_provenance["provenance"] = serde_json::json!([
        "GET",
        [],
        [],
        "text/html; charset=utf-8",
        "identity",
        "2026-10-16T02:00:00Z"
    ]);
    let raw = serde_json::to_string(&original)?;
    let duplicate = raw.replacen("{", "{\"body_bytes\":7,", 1);
    for bad in [
        "[]".to_owned(),
        serde_json::to_string(&array_provenance)?,
        duplicate,
    ] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let leaf = install(
            &store,
            Schema::HtmlResultCaptureV1,
            &bad,
            RetainedCaptureSlotV1::SelfCapture,
            reference(b"context")?,
        )?;
        let before = retained_state(&store)?;
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            verify(&store.snapshot(), &leaf, &mut grant),
            Err(StoreError::Decode { .. }
                | StoreError::EvidenceSubjectAttachment(Error::Corrupt(
                    "external schema requires a JSON object"
                )))
        ));
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}

mod school;
