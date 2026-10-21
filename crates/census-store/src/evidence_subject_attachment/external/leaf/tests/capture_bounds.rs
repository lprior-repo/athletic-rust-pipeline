use super::*;
use crate::evidence_subject_attachment::frozen::FrozenRetainedCaptureSourceV1;
use census_domain::evidence_subject_attachment::AttachmentInventoryV1;

const BODY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../research/sources/national-aggregators/samples/directathletics-tfrrs/indiana-tfrrs-list-5489.html"
));
const BODY_SHA256: &str = "053ff7832326a6d27e2456155db7a9530682393fae94a9fcb9ccc9e88bfc8407";
const KEY: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn retain_controlled_transport(
    store: &Store,
    payload: &serde_json::Value,
    body: &[u8],
) -> TestResult<RetainedCaptureLeafV1> {
    Store::check_retained_capture_entry_v1(Schema::HtmlResultCaptureV1, KEY, payload)?;
    store.journal_done(Schema::HtmlResultCaptureV1.phase(), KEY, payload)?;
    Ok(RetainedCaptureLeafV1 {
        version: 1,
        root: RetainedCaptureRootV1 {
            schema: Schema::HtmlResultCaptureV1,
            key: KEY.into(),
            payload: reference(&serde_json::to_vec(payload)?)?,
        },
        slot: RetainedCaptureSlotV1::SelfCapture,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object: reference(body)?,
    })
}

fn physical_layout(store: &Store) -> TestResult<(usize, usize)> {
    store
        .journal
        .prefix(Store::journal_key(Schema::HtmlResultCaptureV1.phase(), KEY))
        .try_fold((0usize, 0usize), |counts, guard| {
            let (key, value) = guard.into_inner()?;
            check!(value.len() <= crate::MAX_JOURNAL_VALUE_BYTES);
            let rows = counts.0.checked_add(1).ok_or("row overflow")?;
            let bytes = counts
                .1
                .checked_add(key.len())
                .and_then(|n| n.checked_add(value.len()))
                .ok_or("physical byte overflow")?;
            Ok((rows, bytes))
        })
}

fn verify_snapshot(store: &Store, leaf: &RetainedCaptureLeafV1, raw: &[u8]) -> TestResult {
    let view = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let held = view
        .read_retained_capture_root_v1(leaf.root.schema, KEY, &mut grant)?
        .ok_or("controlled transport capture missing")?;
    check!(eq; held.payload().get().as_bytes(), raw);
    check!(eq; held.root(), &leaf.root);
    view.verify_retained_capture_leaf_v1(leaf, &mut grant)?;
    let mut capture_leaf = leaf.clone();
    capture_leaf.kind = RetainedCaptureLeafKindV1::CaptureJson;
    capture_leaf.object = reference(raw)?;
    view.verify_retained_capture_leaf_v1(&capture_leaf, &mut grant)?;
    Ok(())
}

fn frozen_archive(store: &Store, leaf: &RetainedCaptureLeafV1) -> TestResult<Vec<u8>> {
    let mut rows = Vec::new();
    for guard in store
        .journal
        .prefix(Store::journal_key(Schema::HtmlResultCaptureV1.phase(), KEY))
    {
        let (key, value) = guard.into_inner()?;
        let key = key
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let raw: Box<serde_json::value::RawValue> = serde_json::from_slice(&value)?;
        rows.push(format!(r#"{{"key":"{key}","value":{raw}}}"#));
    }
    let layout = physical_layout(store)?;
    let inventory = AttachmentInventoryV1::empty()?;
    let view = store.snapshot();
    let identity = view.store_identity()?;
    let head = view.attachment_public_head()?;
    let root = serde_json::to_string(&leaf.root)?;
    let bytes = format!(
        concat!(
            r#"{{"schema":"retained-evidence-subject-attachment-closure/v1","store":"{}","snapshot_sequence":{},"public_head":{{"sequence":{},"digest":"{}"}},"inventory":{{"version":{},"records":{},"digest":"{}"}},"prefixes":["#,
            r#"{{"kind":"attachment","root":null,"physical_rows":0,"encoded_bytes":0,"rows":[]}},"#,
            r#"{{"kind":"receipts","root":null,"physical_rows":0,"encoded_bytes":0,"rows":[]}},"#,
            r#"{{"kind":"external","root":{},"physical_rows":{},"encoded_bytes":{},"rows":[{}]}}]}}"#
        ),
        identity.as_str(),
        view.sequence(),
        head.sequence,
        head.digest.as_str(),
        inventory.version,
        inventory.records,
        inventory.digest.as_str(),
        root,
        layout.0,
        layout.1,
        rows.join(",")
    );
    Ok(bytes.into_bytes())
}

fn verify_frozen(store: &Store, leaf: &RetainedCaptureLeafV1, raw: &[u8]) -> TestResult {
    let archive = frozen_archive(store, leaf)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let frozen =
        FrozenRetainedCaptureSourceV1::read_bytes(&archive, &reference(&archive)?, &mut grant)?;
    let held = frozen
        .read_retained_capture_root_v1(leaf.root.schema, KEY, &mut grant)?
        .ok_or("frozen controlled transport capture missing")?;
    check!(eq; held.root(), &leaf.root);
    check!(eq; held.payload().get().as_bytes(), raw);
    frozen.verify_retained_capture_leaf_v1(leaf, &mut grant)?;
    let mut capture_leaf = leaf.clone();
    capture_leaf.kind = RetainedCaptureLeafKindV1::CaptureJson;
    capture_leaf.object = reference(raw)?;
    frozen.verify_retained_capture_leaf_v1(&capture_leaf, &mut grant)?;
    Ok(())
}

fn cold_roundtrip_with_controlled_transport(body: &[u8]) -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let declaration = Store::create_current_qualification_v3(&root)?;
    let payload = capture(body)?;
    let raw = serde_json::to_vec(&payload)?;
    check!(raw.len() > 2 * 1024 * 1024);
    let store = open_current_qualification(&root)?;
    check!(eq; store.snapshot().store_identity()?, declaration.store_identity().clone());
    let leaf = retain_controlled_transport(&store, &payload, body)?;
    let layout = physical_layout(&store)?;
    check!(eq; layout.0, 7);
    check!(layout.1 > 3 * 1024 * 1024);
    check!(layout.1 <= 4 * 1024 * 1024);
    let before = retained_state(&store)?;
    verify_snapshot(&store, &leaf, &raw)?;
    verify_frozen(&store, &leaf, &raw)?;
    check!(eq; retained_state(&store)?, before);
    store.flush()?;
    drop(store);
    let store = open_current_qualification(&root)?;
    check!(eq; store.snapshot().store_identity()?, declaration.store_identity().clone());
    check!(eq; retained_state(&store)?, before);
    check!(eq; physical_layout(&store)?, layout);
    verify_snapshot(&store, &leaf, &raw)?;
    verify_frozen(&store, &leaf, &raw)?;
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

fn open_current_qualification(root: &std::path::Path) -> TestResult<Store> {
    let (store, writer) = Store::open_current_qualification_v3(root)?.into_parts();
    drop(writer);
    Ok(store)
}

#[test]
fn genuine_tfrrs_body_with_controlled_transport_preserves_exact_hash_cold() -> TestResult {
    check!(eq; BODY.len(), 1_880_743);
    check!(eq; reference(BODY)?.digest.as_str(), BODY_SHA256);
    cold_roundtrip_with_controlled_transport(BODY)
}

#[test]
fn exact_two_mib_body_with_controlled_transport_survives_all_readers_cold() -> TestResult {
    cold_roundtrip_with_controlled_transport(&vec![b'x'; 2 * 1024 * 1024])
}

#[test]
fn genuine_body_tamper_and_changed_references_refuse_without_store_writes() -> TestResult {
    for case in 0..3 {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let mut body = BODY.to_vec();
        if case == 0 {
            let byte = body.get_mut(4_096).ok_or("sample byte absent")?;
            *byte ^= 1;
        }
        let mut payload = capture(&body)?;
        payload["body_sha256"] = serde_json::Value::String(BODY_SHA256.into());
        let mut leaf = retain_controlled_transport(&store, &payload, BODY)?;
        let expected = match case {
            0 => "external leaf reference differs",
            1 => {
                leaf.object = reference(b"different body")?;
                "external body reference differs from capture"
            }
            _ => {
                leaf.root.payload = reference(b"different root")?;
                "external root payload reference differs"
            }
        };
        let before = retained_state(&store)?;
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            store.snapshot().verify_retained_capture_leaf_v1(&leaf, &mut grant),
            Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(detail))) if detail == expected
        ));
        check!(matches!(
            grant.ensure_available(),
            Err(EvidenceSubjectAttachmentError::Poisoned)
        ));
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}
