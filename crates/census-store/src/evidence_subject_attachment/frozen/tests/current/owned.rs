use super::*;
use crate::evidence_subject_attachment::frozen::FrozenCurrentEvidenceSubjectAttachmentReferencesV1;
use crate::evidence_subject_attachment::tests::workflow::fixture::next_command;
use crate::evidence_subject_attachment::{
    AttachmentContextObjectV1, AttachmentInspectionRequestV1,
    EvidenceSubjectAttachmentPreparationV1 as Preparation, RetainedCaptureLeafKindV1,
    RetainedCaptureLeafV1, RetainedCaptureRootSchemaV1, RetainedCaptureSlotV1,
};
use base64::{engine::general_purpose::STANDARD, Engine};

const KEY: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SCHEMA: RetainedCaptureRootSchemaV1 = RetainedCaptureRootSchemaV1::HtmlResultCaptureV1;

fn revoked_source_history() -> TestResult<(Fixture, Context, crate::RetainedCaptureRootReadV1)> {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("owned-frozen-source-history")?;
    let body = context.objects.get(1).ok_or("body absent")?;
    let payload = serde_json::json!({
        "requested_url": "https://synthetic-attachment.test/results",
        "final_url": "https://synthetic-attachment.test/results", "status": 200,
        "acquired_at": "2026-10-14T00:00:00Z", "body_bytes": body.bytes.len(),
        "body_sha256": body.reference.digest.as_str(), "body_base64": STANDARD.encode(&body.bytes),
        "provenance": {"request_method": "GET", "request_headers": [], "response_header_bytes": [],
            "content_type": "application/json", "body_encoding": "identity", "started_at": "2026-10-14T00:00:00Z"},
    });
    fixture.store.journal_done(SCHEMA.phase(), KEY, &payload)?;
    let original = fixture
        .store
        .snapshot()
        .read_retained_capture_root_v1(SCHEMA, KEY, &mut AttachmentReplayGrantV1::new())?
        .ok_or("original root absent")?;
    let leaf = RetainedCaptureLeafV1 {
        version: 1,
        root: original.root().clone(),
        slot: RetainedCaptureSlotV1::SelfCapture,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object: body.reference.clone(),
    };
    let objects = context
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
    let inspection = fixture.inspect()?;
    let command = context.command("owned-frozen-source-history", &inspection)?;
    let prepared = fixture.store.prepare_evidence_subject_attachment_v1(
        command,
        Preparation::New {
            inspection,
            context: &objects,
        },
    )?;
    written(apply(&fixture.store, prepared)?)?;
    drop(objects);
    let first = first_record(&fixture.store)?;
    let inspection = fixture.store.inspect_evidence_subject_attachment_v1(
        AttachmentInspectionRequestV1::Revoke {
            prior: first.reference(),
        },
    )?;
    let command = next_command(&first, "owned-frozen-source-revoked", &inspection)?;
    let prepared = fixture.store.prepare_evidence_subject_attachment_v1(
        command,
        Preparation::New {
            inspection,
            context: &[],
        },
    )?;
    written(apply(&fixture.store, prepared)?)?;
    Ok((fixture, context, original))
}

#[test]
fn owned_frozen_replay_keeps_exact_revoked_source_history_after_store_close() -> TestResult {
    let (fixture, context, original) = revoked_source_history()?;
    let first = first_record(&fixture.store)?;
    let view = fixture.store.snapshot();
    let mut history_grant = AttachmentReplayGrantV1::new();
    let history = view.read_evidence_subject_attachment_references_v1(&mut history_grant)?;
    let records = history.records().to_vec();
    check!(eq; records.first(), Some(&first));
    drop(history);
    drop(view);
    let body = context.objects.get(1).ok_or("body absent")?;
    let leaf = RetainedCaptureLeafV1 {
        version: 1,
        root: original.root().clone(),
        slot: RetainedCaptureSlotV1::SelfCapture,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object: body.reference.clone(),
    };
    let (bundle, accepted) = capture(&fixture.store)?;
    let store = accepted.store_identity().clone();
    let head = accepted.head().clone();
    let sequence = accepted.snapshot_sequence();
    let mut grant = AttachmentReplayGrantV1::new();
    let owner = FrozenCurrentEvidenceSubjectAttachmentReferencesV1::from_borrowed_bytes(
        bundle.bytes(),
        bundle.reference(),
        accepted,
        &mut grant,
    )?;
    drop(fixture);
    check!(eq; owner.store_identity(), &store);
    check!(eq; owner.accepted_public_head(), &head);
    check!(eq; owner.snapshot_sequence(), sequence);
    check!(eq; owner.retained_records(&mut grant)?, records.as_slice());
    check!(owner
        .bind_exact_subject(&context.subject, &mut grant)?
        .is_none());
    let retained = owner
        .source()
        .read_retained_capture_root_v1(SCHEMA, KEY, &mut grant)?
        .ok_or("frozen root absent")?;
    check!(eq; retained.root(), original.root());
    check!(eq; retained.payload().get().as_bytes(), original.payload().get().as_bytes());
    owner
        .source()
        .verify_retained_capture_leaf_v1(&leaf, &mut grant)?;
    let usage = grant.usage();
    let mut foreign = AttachmentReplayGrantV1::new();
    foreign.charge_bytes(usage.encoded_bytes())?;
    foreign.charge_steps(usage.traversal_steps())?;
    check!(matches!(
        owner.bind_exact_subject(&context.subject, &mut foreign),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Invalid(_)
        )))
    ));
    poisoned(&mut foreign)?;
    check!(eq; owner.retained_records(&mut grant)?, records.as_slice());
    Ok(())
}
