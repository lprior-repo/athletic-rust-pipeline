use super::super::{CapturedExternalPrefix, CapturedJournalRow, FrozenRosterJournalSource};
use crate::evidence_subject_attachment::external::{
    verify_leaf, MAX_RETAINED_CAPTURE_ENVELOPE_BYTES, MAX_RETAINED_CAPTURE_PHYSICAL_ROWS,
};
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::roster_source_issuance::issuance::reader::current;
use crate::{
    RetainedCaptureLeafKindV1, RetainedCaptureLeafV1, RetainedCaptureRootSchemaV1,
    RetainedCaptureRootV1, RetainedCaptureSlotV1, Store, StoreError,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
};
use sha2::{Digest, Sha256};

const BODY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../research/sources/national-aggregators/samples/directathletics-tfrrs/indiana-tfrrs-list-5489.html"
));
const BODY_DIGEST: &str = "053ff7832326a6d27e2456155db7a9530682393fae94a9fcb9ccc9e88bfc8407";
const SCHEMA: RetainedCaptureRootSchemaV1 = RetainedCaptureRootSchemaV1::HtmlResultCaptureV1;

fn reference(bytes: &[u8]) -> TestResult<AttachmentObjectReferenceV1> {
    Ok(AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(bytes)))?,
        bytes: u64::try_from(bytes.len())?,
    })
}

fn retain_controlled_transport(
    store: &Store,
    body: &[u8],
) -> TestResult<(RetainedCaptureRootV1, Vec<u8>)> {
    let payload = serde_json::json!({
        "requested_url":"https://example.test/controlled-transport",
        "final_url":"https://example.test/controlled-transport",
        "status":200,"acquired_at":"2026-10-16T02:00:00Z","body_bytes":body.len(),
        "body_sha256":reference(body)?.digest.as_str(),"body_base64":STANDARD.encode(body),
        "provenance":{"request_method":"GET","request_headers":[],
            "response_header_bytes":[],"content_type":"text/html; charset=utf-8",
            "body_encoding":"identity","started_at":"2026-10-16T02:00:00Z"}
    });
    let raw = serde_json::to_vec(&payload)?;
    let payload = reference(&raw)?;
    let key = payload.digest.as_str().to_owned();
    let value: serde_json::Value = serde_json::from_slice(&raw)?;
    Store::check_retained_capture_entry_v1(SCHEMA, &key, &value)?;
    store.journal_done(SCHEMA.phase(), &key, &value)?;
    Ok((
        RetainedCaptureRootV1 {
            schema: SCHEMA,
            key,
            payload,
        },
        raw,
    ))
}

fn physical_bytes(group: &CapturedExternalPrefix) -> TestResult<usize> {
    group.rows.iter().try_fold(0usize, |total, row| {
        total
            .checked_add(row.key.len())
            .and_then(|next| next.checked_add(row.value.len()))
            .ok_or_else(|| "physical byte overflow".into())
    })
}

fn verify_parity(
    store: &Store,
    root: &RetainedCaptureRootV1,
    raw: &[u8],
    body: &[u8],
) -> TestResult {
    let before = retained_state(store)?;
    let view = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let held = view
        .read_retained_capture_root_v1(root.schema, &root.key, &mut grant)?
        .ok_or("current retained capture missing")?;
    check!(eq; held.root(), root);
    check!(eq; held.payload().get().as_bytes(), raw);
    let group = current::capture(&view, root.clone(), &mut grant)?;
    check!(eq; u64::try_from(group.rows.len())?, MAX_RETAINED_CAPTURE_PHYSICAL_ROWS);
    check!(physical_bytes(&group)? > 3 * 1024 * 1024);
    check!(physical_bytes(&group)? <= 4 * 1024 * 1024);
    check!(group
        .rows
        .iter()
        .all(|row| row.value.len() <= crate::MAX_JOURNAL_VALUE_BYTES));
    let groups = vec![group];
    let source = FrozenRosterJournalSource::new(&groups, &mut grant)?;
    let frozen = source
        .read_root(root.schema, &root.key, &mut grant)?
        .ok_or("frozen retained capture missing")?;
    check!(eq; frozen.root(), held.root());
    check!(eq; frozen.payload().get(), held.payload().get());
    let leaf = RetainedCaptureLeafV1 {
        version: 1,
        root: root.clone(),
        slot: RetainedCaptureSlotV1::SelfCapture,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object: reference(body)?,
    };
    view.verify_retained_capture_leaf_v1(&leaf, &mut grant)?;
    verify_leaf(&source, &leaf, &mut grant)?;
    check!(grant.ensure_available().is_ok());
    check!(eq; retained_state(store)?, before);
    Ok(())
}

#[test]
fn genuine_body_with_controlled_transport_has_current_frozen_parity_after_cold_open() -> TestResult
{
    check!(eq; BODY.len(), 1_880_743);
    check!(eq; reference(BODY)?.digest.as_str(), BODY_DIGEST);
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let (root, raw) = retain_controlled_transport(&store, BODY)?;
    check!(raw.len() > 2 * 1024 * 1024);
    check!(raw.len() < MAX_RETAINED_CAPTURE_ENVELOPE_BYTES);
    let before = retained_state(&store)?;
    verify_parity(&store, &root, &raw, BODY)?;
    store.flush()?;
    drop(store);
    let cold = Store::open(directory.path())?;
    check!(eq; retained_state(&cold)?, before);
    verify_parity(&cold, &root, &raw, BODY)?;
    Ok(())
}

fn fault(group: &mut CapturedExternalPrefix, mode: usize) -> TestResult {
    match mode {
        0 => group.rows.push(CapturedJournalRow {
            key: crate::journal::chunk_key(
                &Store::journal_key(group.root.schema.phase(), &group.root.key),
                6,
            )?,
            value: b"null".to_vec(),
        }),
        1 => group
            .rows
            .last_mut()
            .ok_or("last capture row absent")?
            .value
            .resize(crate::MAX_JOURNAL_VALUE_BYTES + 1, b' '),
        _ => pad_one_past_physical_limit(group)?,
    }
    Ok(())
}

fn pad_one_past_physical_limit(group: &mut CapturedExternalPrefix) -> TestResult {
    let limit = 4 * 1024 * 1024;
    let mut remaining = (limit + 1usize)
        .checked_sub(physical_bytes(group)?)
        .ok_or("fixture already exceeds physical byte limit")?;
    for row in group.rows.iter_mut().rev() {
        let spare = crate::MAX_JOURNAL_VALUE_BYTES
            .checked_sub(row.value.len())
            .ok_or("fixture record already oversized")?;
        let added = remaining.min(spare);
        let length = row
            .value
            .len()
            .checked_add(added)
            .ok_or("record overflow")?;
        row.value.resize(length, b' ');
        remaining = remaining.checked_sub(added).ok_or("padding underflow")?;
        if remaining == 0 {
            break;
        }
    }
    check!(eq; remaining, 0);
    check!(eq; physical_bytes(group)?, limit + 1);
    Ok(())
}

fn require_refusal(error: StoreError, mode: usize) -> TestResult {
    match mode {
        0 => check!(matches!(
            error,
            StoreError::BudgetExceeded {
                what: "journal physical rows",
                needed: 8,
                available: 7
            }
        )),
        1 => check!(matches!(error, StoreError::RowTooLarge { bytes, max }
            if bytes == crate::MAX_JOURNAL_VALUE_BYTES + 1 && max == crate::MAX_JOURNAL_VALUE_BYTES)),
        _ => check!(matches!(error, StoreError::BudgetExceeded {
            what: "journal encoded bytes", needed, available
        } if needed == 4 * 1024 * 1024 + 1 && available == 4 * 1024 * 1024)),
    }
    Ok(())
}

#[test]
fn current_and_frozen_keep_row_record_and_physical_byte_limits() -> TestResult {
    for mode in 0..3 {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let (root, _) = retain_controlled_transport(&store, BODY)?;
        let mut grant = AttachmentReplayGrantV1::new();
        let mut group = current::capture(&store.snapshot(), root.clone(), &mut grant)?;
        fault(&mut group, mode)?;
        for row in &group.rows {
            store.journal.insert(&row.key, &row.value)?;
        }
        let before = retained_state(&store)?;
        let mut current_grant = AttachmentReplayGrantV1::new();
        let error = current::capture(&store.snapshot(), root, &mut current_grant)
            .err()
            .ok_or("current physical fault was accepted")?;
        require_refusal(error, mode)?;
        let mut frozen_grant = AttachmentReplayGrantV1::new();
        let error = FrozenRosterJournalSource::new(&[group], &mut frozen_grant)
            .err()
            .ok_or("frozen physical fault was accepted")?;
        require_refusal(error, mode)?;
        check!(frozen_grant.ensure_available().is_err());
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}

#[test]
fn one_past_logical_capture_bound_and_exhausted_original_grant_refuse() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let (root, _) = retain_controlled_transport(&store, BODY)?;
    let before = retained_state(&store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let mut group = current::capture(&store.snapshot(), root.clone(), &mut grant)?;
    group.root.payload.bytes = u64::try_from(MAX_RETAINED_CAPTURE_ENVELOPE_BYTES)? + 1;
    let mut frozen_grant = AttachmentReplayGrantV1::new();
    check!(FrozenRosterJournalSource::new(&[group], &mut frozen_grant).is_err());
    check!(frozen_grant.ensure_available().is_err());
    let mut exhausted = AttachmentReplayGrantV1::new();
    exhausted.charge_bytes(AttachmentReplayGrantV1::MAX_ENCODED_BYTES)?;
    let consumed = exhausted.usage().encoded_bytes();
    let groups = vec![current::capture(&store.snapshot(), root, &mut grant)?];
    check!(FrozenRosterJournalSource::new(&groups, &mut exhausted).is_err());
    check!(eq; exhausted.usage().encoded_bytes(), consumed);
    check!(exhausted.ensure_available().is_err());
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn exact_body_bound_replays_and_one_over_refuses_current_and_frozen() -> TestResult {
    for length in [2 * 1024 * 1024, 2 * 1024 * 1024 + 1] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let body = vec![b'x'; length];
        let (root, raw) = retain_controlled_transport(&store, &body)?;
        if length == 2 * 1024 * 1024 {
            verify_parity(&store, &root, &raw, &body)?;
            continue;
        }
        let before = retained_state(&store)?;
        let mut current_grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            store.snapshot().read_retained_capture_root_v1(
                root.schema,
                &root.key,
                &mut current_grant
            ),
            Err(StoreError::EvidenceSubjectAttachment(
                crate::EvidenceSubjectAttachmentStoreError::Corrupt("external capture body bounds")
            ))
        ));
        check!(current_grant.ensure_available().is_err());
        let mut grant = AttachmentReplayGrantV1::new();
        let groups = vec![current::capture(
            &store.snapshot(),
            root.clone(),
            &mut grant,
        )?];
        let source = FrozenRosterJournalSource::new(&groups, &mut grant)?;
        check!(matches!(
            source.read_root(root.schema, &root.key, &mut grant),
            Err(StoreError::EvidenceSubjectAttachment(
                crate::EvidenceSubjectAttachmentStoreError::Corrupt("external capture body bounds")
            ))
        ));
        check!(grant.ensure_available().is_err());
        let consumed = grant.usage();
        check!(matches!(
            source.read_root(root.schema, &root.key, &mut grant),
            Err(StoreError::EvidenceSubjectAttachment(
                crate::EvidenceSubjectAttachmentStoreError::Domain(
                    census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned
                )
            ))
        ));
        check!(eq; grant.usage(), consumed);
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}
