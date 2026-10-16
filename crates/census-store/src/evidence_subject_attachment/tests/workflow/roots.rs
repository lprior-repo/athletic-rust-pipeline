use super::fixture::*;
use crate::evidence_subject_attachment::{
    AttachmentContextObjectV1, EvidenceSubjectAttachmentPreparationV1 as Preparation,
    EvidenceSubjectAttachmentStoreError as AttachmentError, RetainedCaptureLeafKindV1,
    RetainedCaptureLeafV1, RetainedCaptureRootSchemaV1, RetainedCaptureSlotV1,
};
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::{Store, StoreError};
use base64::{engine::general_purpose::STANDARD, Engine};
use census_domain::evidence_subject_attachment::*;

const KEY: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const SCHEMA: RetainedCaptureRootSchemaV1 = RetainedCaptureRootSchemaV1::HtmlResultCaptureV1;

fn capture(body: &InlineObject, status: u16) -> serde_json::Value {
    serde_json::json!({
        "requested_url": "https://synthetic-attachment.test/results",
        "final_url": "https://synthetic-attachment.test/results", "status": status,
        "acquired_at": "2026-10-14T00:00:00Z", "body_bytes": body.bytes.len(),
        "body_sha256": body.reference.digest.as_str(), "body_base64": STANDARD.encode(&body.bytes),
        "provenance": {"request_method": "GET", "request_headers": [], "response_header_bytes": [],
            "content_type": "application/json", "body_encoding": "identity", "started_at": "2026-10-14T00:00:00Z"},
    })
}

fn retain_body(store: &Store, body: &InlineObject) -> TestResult<RetainedCaptureLeafV1> {
    store.journal_done(SCHEMA.phase(), KEY, &capture(body, 200))?;
    let view = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let root = view
        .read_retained_capture_root_v1(SCHEMA, KEY, &mut grant)?
        .ok_or("controlled retained root absent")?;
    let leaf = RetainedCaptureLeafV1 {
        version: 1,
        root: root.root().clone(),
        slot: RetainedCaptureSlotV1::SelfCapture,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object: body.reference.clone(),
    };
    view.verify_retained_capture_leaf_v1(&leaf, &mut grant)?;
    Ok(leaf)
}

fn borrowed_with_leaf<'a>(
    context: &'a Context,
    leaf: &'a RetainedCaptureLeafV1,
) -> Vec<AttachmentContextObjectV1<'a>> {
    context
        .objects
        .iter()
        .map(|object| {
            if object.reference == leaf.object {
                AttachmentContextObjectV1::Retained(leaf)
            } else {
                object.borrowed()
            }
        })
        .collect()
}

#[test]
fn generic_root_replacement_after_preparation_is_rejected_by_final_guarded_closure_read(
) -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("changed-root-before-apply")?;
    let body = context.objects.get(1).ok_or("controlled body missing")?;
    let leaf = retain_body(&fixture.store, body)?;
    let inspection = fixture.inspect()?;
    let command = context.command("changed-root-before-apply", &inspection)?;
    let prepared = fixture.store.prepare_evidence_subject_attachment_v1(
        command,
        Preparation::New {
            inspection,
            context: &borrowed_with_leaf(&context, &leaf),
        },
    )?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    let evidence = fixture.store.evidence_generation();
    let derived = fixture.store.derived_generation();
    fixture
        .store
        .journal_done(SCHEMA.phase(), KEY, &capture(body, 201))?;
    check!(eq; fixture.store.evidence_generation(), evidence);
    check!(eq; fixture.store.derived_generation(), derived);
    let before = retained_state(&fixture.store)?;
    check!(matches!(
        fixture
            .store
            .apply_evidence_subject_attachment_v1(prepared, &approval, &digest),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Corrupt("external root payload reference differs")
        ))
    ));
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn held_attachment_read_keeps_original_external_closure_when_current_root_is_replaced() -> TestResult
{
    let fixture = Fixture::new()?;
    let context = Context::synthetic("held-root-read")?;
    let body = context.objects.get(1).ok_or("controlled body missing")?;
    let leaf = retain_body(&fixture.store, body)?;
    let inspection = fixture.inspect()?;
    let command = context.command("held-root-read", &inspection)?;
    let prepared = fixture.store.prepare_evidence_subject_attachment_v1(
        command.clone(),
        Preparation::New {
            inspection,
            context: &borrowed_with_leaf(&context, &leaf),
        },
    )?;
    let receipt = written(apply(&fixture.store, prepared)?)?;
    let held = fixture.store.snapshot();
    let original_record = first_record(&fixture.store)?;
    let evidence = fixture.store.evidence_generation();
    let derived = fixture.store.derived_generation();
    fixture
        .store
        .journal_done(SCHEMA.phase(), KEY, &capture(body, 201))?;
    check!(eq; fixture.store.evidence_generation(), evidence);
    check!(eq; fixture.store.derived_generation(), derived);
    let before = retained_state(&fixture.store)?;
    let mut original_grant = AttachmentReplayGrantV1::new();
    let retained = held.read_evidence_subject_attachment_references_v1(&mut original_grant)?;
    check!(eq; retained.records(), std::slice::from_ref(&original_record));
    check!(eq; retained.receipts(), std::slice::from_ref(&receipt));
    let mut current_grant = AttachmentReplayGrantV1::new();
    let current = fixture.store.snapshot();
    check!(matches!(
        current.read_evidence_subject_attachment_references_v1(&mut current_grant),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Corrupt("external root payload reference differs")
        ))
    ));
    let poisoned_usage = current_grant.usage();
    check!(matches!(
        current.read_evidence_subject_attachment_references_v1(&mut current_grant),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Domain(EvidenceSubjectAttachmentError::Poisoned)
        ))
    ));
    check!(eq; current_grant.usage(), poisoned_usage);
    check!(eq; retained_state(&fixture.store)?, before);
    check!(eq; retained.records().first().ok_or("held record absent")?.command, command);
    Ok(())
}
