use super::*;
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::Store;
use census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError;

const KEY: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SCHEMA: RetainedCaptureRootSchemaV1 = RetainedCaptureRootSchemaV1::HtmlResultCaptureV1;

fn payload() -> serde_json::Value {
    serde_json::json!({
        "requested_url":"https://example.test/source","final_url":"https://example.test/source",
        "status":200,"acquired_at":"2026-10-16T02:00:00Z","body_bytes":1,
        "body_sha256":"2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881",
        "body_base64":"eA==","provenance":{"request_method":"GET","request_headers":[],
        "response_header_bytes":[],"content_type":"text/html; charset=utf-8","body_encoding":"identity","started_at":"2026-10-16T02:00:00Z"}
    })
}

#[test]
fn public_owned_reader_retains_exact_bytes_charge_after_holder_drop() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.journal_done(SCHEMA.phase(), KEY, &payload())?;
    let expected = serde_json::to_vec(&payload())?;
    let before = retained_state(&store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let held = store
        .snapshot()
        .read_retained_capture_root_v1(SCHEMA, KEY, &mut grant)?
        .ok_or("capture absent")?;
    check!(eq; held.payload().get().as_bytes(), expected.as_slice());
    check!(eq; held.root().key, KEY);
    check!(eq; held.root().schema, SCHEMA);
    check!(eq; held.root().payload.bytes, u64::try_from(expected.len())?);
    check!(eq; held.root().payload.digest.as_str(), format!("{:x}", Sha256::digest(&expected)));
    check!(eq; grant.usage().traversal_steps(), 46);
    check!(eq; grant.usage().encoded_bytes(), expected.len());
    drop(held);
    check!(eq; grant.usage().encoded_bytes(), expected.len());
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn public_owned_byte_refusal_never_issues_holder_or_resets_grant() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.journal_done(SCHEMA.phase(), KEY, &payload())?;
    let bytes = serde_json::to_vec(&payload())?.len();
    let before = retained_state(&store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let prior = (AttachmentReplayGrantV1::MAX_ENCODED_BYTES)
        .checked_sub(bytes)
        .and_then(|n| n.checked_add(1))
        .ok_or("fixture underflow")?;
    grant.charge_bytes(prior)?;
    check!(matches!(
        store
            .snapshot()
            .read_retained_capture_root_v1(SCHEMA, KEY, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate encoded bytes",
                needed,
                available,
            }
        ))) if available == AttachmentReplayGrantV1::MAX_ENCODED_BYTES
            && needed.checked_sub(available) == Some(1)
    ));
    check!(eq; grant.usage().encoded_bytes(), prior);
    check!(eq; grant.usage().traversal_steps(), 46);
    check!(matches!(
        store
            .snapshot()
            .read_retained_capture_root_v1(SCHEMA, KEY, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Poisoned
        )))
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn missing_is_explicit_and_prefix_extension_or_invalid_key_refuses() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let mut absent = AttachmentReplayGrantV1::new();
    check!(store
        .snapshot()
        .read_retained_capture_root_v1(SCHEMA, KEY, &mut absent)?
        .is_none());
    check!(eq; absent.usage().traversal_steps(), 3);
    check!(eq; absent.usage().encoded_bytes(), 0);
    let mut invalid = AttachmentReplayGrantV1::new();
    check!(matches!(
        store
            .snapshot()
            .read_retained_capture_root_v1(SCHEMA, "bad-key", &mut invalid),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "external root descriptor bounds"
        )))
    ));
    store.journal_done(SCHEMA.phase(), &format!("{KEY}extension"), &payload())?;
    let before = retained_state(&store)?;
    let mut foreign = AttachmentReplayGrantV1::new();
    check!(matches!(
        store
            .snapshot()
            .read_retained_capture_root_v1(SCHEMA, KEY, &mut foreign),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "external root prefix has a foreign root"
        )))
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn pinned_revalidation_uses_current_root_even_when_input_generations_match() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store.journal_done(SCHEMA.phase(), KEY, &payload())?;
    let original = store.snapshot();
    let mut prepare_grant = AttachmentReplayGrantV1::new();
    let holder = original
        .read_retained_capture_root_v1(SCHEMA, KEY, &mut prepare_grant)?
        .ok_or("root absent")?;
    let descriptor = holder.root().clone();
    drop(holder);
    let evidence = store.evidence_generation();
    let derived = store.derived_generation();
    let mut replacement = payload();
    replacement["status"] = serde_json::json!(201);
    store.journal_done(SCHEMA.phase(), KEY, &replacement)?;
    check!(eq; store.evidence_generation(), evidence);
    check!(eq; store.derived_generation(), derived);
    let before = retained_state(&store)?;
    let mut old_grant = AttachmentReplayGrantV1::new();
    let observed = with_checked(&original, &descriptor, &mut old_grant, |raw, _, _| {
        Ok(raw.get().to_owned())
    })?;
    check!(eq; serde_json::from_str::<serde_json::Value>(&observed)?, payload());
    let mut current_grant = AttachmentReplayGrantV1::new();
    {
        let _guard = store.lock_appends();
        check!(matches!(
            with_checked(
                &store.snapshot_locked(),
                &descriptor,
                &mut current_grant,
                |_, _, _| Ok(())
            ),
            Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(_)))
        ));
    }
    check!(eq; retained_state(&store)?, before);
    Ok(())
}
