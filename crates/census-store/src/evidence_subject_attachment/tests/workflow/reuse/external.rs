use super::super::fixture::*;
use super::support::{refuses_at_boundary, Boundary};
use crate::evidence_subject_attachment::{
    AttachmentContextObjectV1, EvidenceSubjectAttachmentPreparationV1 as Preparation,
    RetainedCaptureLeafKindV1, RetainedCaptureLeafV1, RetainedCaptureRootSchemaV1,
    RetainedCaptureSlotV1,
};
use crate::public_identity_tests::support::TestResult;
use base64::{engine::general_purpose::STANDARD, Engine};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;

const KEY: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const SCHEMA: RetainedCaptureRootSchemaV1 = RetainedCaptureRootSchemaV1::HtmlResultCaptureV1;

fn capture(body: &InlineObject, status: u16) -> serde_json::Value {
    serde_json::json!({
        "requested_url":"https://synthetic-owner-reuse.test/results",
        "final_url":"https://synthetic-owner-reuse.test/results", "status":status,
        "acquired_at":"2026-10-14T00:00:00Z", "body_bytes":body.bytes.len(),
        "body_sha256":body.reference.digest.as_str(), "body_base64":STANDARD.encode(&body.bytes),
        "provenance":{"request_method":"GET", "request_headers":[], "response_header_bytes":[],
            "content_type":"application/json", "body_encoding":"identity", "started_at":"2026-10-14T00:00:00Z"},
    })
}

fn external_case(boundary: Boundary) -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("owner-reuse-retained-original")?;
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
    let command = context.command("owner-reuse-retained-original", &inspection)?;
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
    refuses_at_boundary(
        &fixture,
        boundary,
        |store| {
            let evidence = store.evidence_generation();
            let derived = store.derived_generation();
            store.journal_done(SCHEMA.phase(), KEY, &capture(body, 201))?;
            check!(eq; store.evidence_generation(), evidence);
            check!(eq; store.derived_generation(), derived);
            let raw = store
                .snapshot()
                .journal_payload_raw_bounded(
                    "evidence_subject",
                    "attachment/v1/record/00000000000000000001",
                    crate::JournalReadBudget::new(4 * 1024 * 1024)?,
                )?
                .ok_or("unchanged attachment root absent")?;
            let payload: serde_json::Value = serde_json::from_str(raw.get())?;
            check!(eq; payload.get("record"), Some(&serde_json::to_value(&original)?));
            Ok(())
        },
        "external root payload reference differs",
    )
}

#[test]
fn publicly_replaced_prior_external_root_after_inspection_is_reverified_before_preparation(
) -> TestResult {
    external_case(Boundary::Preparation)
}

#[test]
fn publicly_replaced_prior_external_root_after_preparation_is_reverified_before_application(
) -> TestResult {
    external_case(Boundary::Application)
}
