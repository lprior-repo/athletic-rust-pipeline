use super::*;
use crate::evidence_subject_attachment::RetainedCaptureRootSchemaV1;
use crate::evidence_subject_attachment::RetainedCaptureRootV1;
use crate::journal::{chunk_key, chunk_value, manifest_value};
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::Store;
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentObjectReferenceV1, EvidenceSubjectAttachmentError,
};
use sha2::{Digest, Sha256};

const AT: &str = "2026-10-16T02:00:00Z";

fn descriptor(raw: &[u8]) -> TestResult<RetainedCaptureRootV1> {
    Ok(RetainedCaptureRootV1 {
        schema: RetainedCaptureRootSchemaV1::HtmlResultCaptureV1,
        key: "a".repeat(64),
        payload: AttachmentObjectReferenceV1 {
            digest: AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(raw)))?,
            bytes: u64::try_from(raw.len())?,
        },
    })
}

fn install(
    store: &Store,
    root: &RetainedCaptureRootV1,
    raw: &str,
    chunked: bool,
) -> TestResult<u64> {
    let encoded = format!(r#"{{"key":"{}","at":"{AT}","payload":{raw}}}"#, root.key);
    let key = Store::journal_key(root.schema.phase(), &root.key);
    let rows = if chunked {
        vec![
            (key.clone(), manifest_value(&root.key, 1, encoded.len())?),
            (
                chunk_key(&key, 0)?,
                chunk_value(&root.key, 0, 1, encoded.as_bytes())?,
            ),
        ]
    } else {
        vec![(key, encoded.into_bytes())]
    };
    rows.into_iter().try_fold(0u64, |bytes, (key, raw)| {
        let added = u64::try_from(key.len().checked_add(raw.len()).ok_or("fixture overflow")?)?;
        store.journal.insert(key, raw)?;
        bytes
            .checked_add(added)
            .ok_or_else(|| "fixture overflow".into())
    })
}

#[test]
fn exact_raw_payload_and_independent_fixed_pass_cost_hold_inline_and_chunked() -> TestResult {
    for (chunked, rows, expected_steps) in [(false, 1usize, 42usize), (true, 2, 63)] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let raw = r#"{"full_source":"retained original context"}"#;
        let root = descriptor(raw.as_bytes())?;
        install(&store, &root, raw, chunked)?;
        let before = retained_state(&store)?;
        let mut grant = AttachmentReplayGrantV1::new();
        let envelope =
            read(&store.snapshot(), root.schema, &root.key, &mut grant)?.ok_or("root absent")?;
        check!(eq; envelope.payload().get(), raw);
        check!(eq; envelope.at(), AT);
        check!(eq; expected_steps, 1 + 2 + 4 * rows + 1 + 17 * (rows + 1));
        check!(eq; grant.usage().traversal_steps(), expected_steps);
        check!(eq; grant.usage().encoded_bytes(), 0);
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}

#[test]
fn decoder_pass_admission_refuses_before_read_and_remains_poisoned() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let root = descriptor(b"null")?;
    install(&store, &root, "null", false)?;
    let before = retained_state(&store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_steps(
        AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
            .checked_sub(8)
            .ok_or("decoder admission steps absent")?,
    )?;
    let result = read(&store.snapshot(), root.schema, &root.key, &mut grant);
    check!(matches!(
        result,
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Capacity {
                resource: "aggregate traversal steps",
                needed,
                available
            }
        ))) if available == AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
            && needed.checked_sub(available) == Some(2 * DECODER_PASS_SLOTS)
    ));
    check!(eq; grant.usage().traversal_steps(), AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS);
    check!(matches!(
        read(&store.snapshot(), root.schema, &root.key, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            EvidenceSubjectAttachmentError::Poisoned
        )))
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn complete_tiny_physical_inventory_caps_manifest_before_reassembly() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let root = descriptor(b"null")?;
    install(&store, &root, "null", true)?;
    let key = Store::journal_key(root.schema.phase(), &root.key);
    store
        .journal
        .insert(key, manifest_value(&root.key, 1, 4_096)?)?;
    let physical_bytes =
        store
            .journal
            .iter()
            .try_fold(0usize, |sum, guard| -> TestResult<usize> {
                let (key, value) = guard.into_inner()?;
                sum.checked_add(key.len())
                    .and_then(|n| n.checked_add(value.len()))
                    .ok_or_else(|| "fixture overflow".into())
            })?;
    check!(physical_bytes < 4_096);
    let before = retained_state(&store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(
        matches!(read(&store.snapshot(), root.schema, &root.key, &mut grant),
        Err(StoreError::JournalReadLimit { what: "encoded envelope", needed: 4_096, max, .. }) if max == physical_bytes)
    );
    check!(eq; grant.usage().traversal_steps(), 63);
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}
