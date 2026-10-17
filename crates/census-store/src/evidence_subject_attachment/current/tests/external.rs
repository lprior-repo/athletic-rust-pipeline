use super::*;
use crate::evidence_subject_attachment::{
    AttachmentContextObjectV1, RetainedCaptureLeafKindV1, RetainedCaptureLeafV1,
    RetainedCaptureRootSchemaV1, RetainedCaptureSlotV1,
};
use crate::Store;
use base64::{engine::general_purpose::STANDARD, Engine};

const KEY: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const SCHEMA: RetainedCaptureRootSchemaV1 = RetainedCaptureRootSchemaV1::HtmlResultCaptureV1;

fn capture(body: &InlineObject, status: u16) -> serde_json::Value {
    serde_json::json!({
        "requested_url":"https://synthetic-current-owner.test/results",
        "final_url":"https://synthetic-current-owner.test/results", "status":status,
        "acquired_at":"2026-10-14T00:00:00Z", "body_bytes":body.bytes.len(),
        "body_sha256":body.reference.digest.as_str(), "body_base64":STANDARD.encode(&body.bytes),
        "provenance":{"request_method":"GET", "request_headers":[], "response_header_bytes":[],
            "content_type":"application/json", "body_encoding":"identity", "started_at":"2026-10-14T00:00:00Z"},
    })
}

fn committed_external() -> TestResult<(Fixture, Context)> {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("current-retained-original")?;
    let body = context
        .objects
        .iter()
        .find(|object| object.reference.digest == context.subject.body)
        .ok_or("retained original body absent")?;
    fixture
        .store
        .journal_done(SCHEMA.phase(), KEY, &capture(body, 200))?;
    let view = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let root = view
        .read_retained_capture_root_v1(SCHEMA, KEY, &mut grant)?
        .ok_or("root absent")?;
    let leaf = RetainedCaptureLeafV1 {
        version: 1,
        root: root.root().clone(),
        slot: RetainedCaptureSlotV1::SelfCapture,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object: body.reference.clone(),
    };
    view.verify_retained_capture_leaf_v1(&leaf, &mut grant)?;
    drop(root);
    drop(view);
    let inspection = fixture.inspect()?;
    let command = context.command("current-retained-original", &inspection)?;
    let borrowed = context
        .objects
        .iter()
        .map(|object| {
            if object.reference == leaf.object {
                AttachmentContextObjectV1::Retained(&leaf)
            } else {
                object.borrowed()
            }
        })
        .collect::<Vec<_>>();
    written(apply(
        &fixture.store,
        fixture.store.prepare_evidence_subject_attachment_v1(
            command,
            Preparation::New {
                inspection,
                context: &borrowed,
            },
        )?,
    )?)?;
    drop(borrowed);
    Ok((fixture, context))
}

#[test]
fn changed_original_root_is_visible_only_to_new_current_readers_and_refuses_there() -> TestResult {
    let (fixture, context) = committed_external()?;
    let old_view = fixture.store.snapshot();
    let mut old_grant = AttachmentReplayGrantV1::new();
    let old_owner =
        old_view.read_current_evidence_subject_attachment_references_v1(&mut old_grant)?;
    let body = context
        .objects
        .iter()
        .find(|object| object.reference.digest == context.subject.body)
        .ok_or("retained body absent")?;
    fixture
        .store
        .journal_done(SCHEMA.phase(), KEY, &capture(body, 201))?;
    let before = retained_state(&fixture.store)?;
    let bound = old_owner
        .bind_exact_subject(&context.subject, &mut old_grant)?
        .ok_or("old-view subject absent")?;
    bound.require_original_grant(&mut old_grant)?;
    check!(std::ptr::eq(bound.snapshot(), &old_view));
    let view = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        view.read_current_evidence_subject_attachment_references_v1(&mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "external root payload reference differs"
        )))
    ));
    poisoned(&mut grant)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn missing_original_root_prevents_current_owner_construction_and_keeps_all_state() -> TestResult {
    let (fixture, _context) = committed_external()?;
    fixture
        .store
        .journal
        .remove(Store::journal_key(SCHEMA.phase(), KEY))?;
    let before = retained_state(&fixture.store)?;
    let view = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        view.read_current_evidence_subject_attachment_references_v1(&mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "external root is absent or ambiguous"
        )))
    ));
    poisoned(&mut grant)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}
