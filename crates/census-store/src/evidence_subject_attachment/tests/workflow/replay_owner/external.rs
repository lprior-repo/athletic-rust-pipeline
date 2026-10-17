use super::*;
use crate::evidence_subject_attachment::{
    AttachmentContextObjectV1, RetainedCaptureLeafKindV1, RetainedCaptureLeafV1,
    RetainedCaptureRootSchemaV1, RetainedCaptureSlotV1,
};
use base64::{engine::general_purpose::STANDARD, Engine};

const KEY: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const SCHEMA: RetainedCaptureRootSchemaV1 = RetainedCaptureRootSchemaV1::HtmlResultCaptureV1;

fn capture(body: &InlineObject, status: u16) -> serde_json::Value {
    serde_json::json!({
        "requested_url":"https://synthetic-replay-owner.test/results",
        "final_url":"https://synthetic-replay-owner.test/results", "status":status,
        "acquired_at":"2026-10-14T00:00:00Z", "body_bytes":body.bytes.len(),
        "body_sha256":body.reference.digest.as_str(), "body_base64":STANDARD.encode(&body.bytes),
        "provenance":{"request_method":"GET", "request_headers":[], "response_header_bytes":[],
            "content_type":"application/json", "body_encoding":"identity", "started_at":"2026-10-14T00:00:00Z"},
    })
}

#[test]
fn replay_owner_public_external_root_replacement_is_reverified_before_receipt_return() -> TestResult
{
    let fixture = Fixture::new()?;
    let context = Context::synthetic("replay-owner-retained-original")?;
    let body = context
        .objects
        .iter()
        .find(|object| object.reference.digest == context.subject.body)
        .ok_or("original body material absent")?;
    fixture
        .store
        .journal_done(SCHEMA.phase(), KEY, &capture(body, 200))?;
    let leaf = {
        let view = fixture.store.snapshot();
        let mut grant = AttachmentReplayGrantV1::new();
        let root = view
            .read_retained_capture_root_v1(SCHEMA, KEY, &mut grant)?
            .ok_or("original root absent")?;
        let leaf = RetainedCaptureLeafV1 {
            version: 1,
            root: root.root().clone(),
            slot: RetainedCaptureSlotV1::SelfCapture,
            kind: RetainedCaptureLeafKindV1::OriginalBody,
            object: body.reference.clone(),
        };
        view.verify_retained_capture_leaf_v1(&leaf, &mut grant)?;
        leaf
    };
    let inspection = fixture.inspect()?;
    let command = context.command("replay-owner-retained-original", &inspection)?;
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
    let prepared = fixture.store.prepare_evidence_subject_attachment_v1(
        command,
        Preparation::New {
            inspection,
            context: &borrowed,
        },
    )?;
    written(apply(&fixture.store, prepared)?)?;
    let original = first_record(&fixture.store)?;
    let replay = fixture.store.prepare_evidence_subject_attachment_v1(
        original.command.clone(),
        Preparation::ReplayOnly,
    )?;
    let attachment_key = Store::journal_key("evidence_subject", RECORD_KEY);
    let root = fixture
        .store
        .journal
        .get(&attachment_key)?
        .ok_or("attachment root absent")?;
    let evidence = fixture.store.evidence_generation();
    let derived = fixture.store.derived_generation();
    fixture
        .store
        .journal_done(SCHEMA.phase(), KEY, &capture(body, 201))?;
    check!(eq; fixture.store.evidence_generation(), evidence);
    check!(eq; fixture.store.derived_generation(), derived);
    check!(eq; fixture.store.journal.get(&attachment_key)?.ok_or("attachment root absent")?, root);
    refuse_corrupt(
        &fixture.store,
        replay,
        "external root payload reference differs",
    )
}
