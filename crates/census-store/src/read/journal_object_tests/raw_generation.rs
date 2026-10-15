use super::*;
use crate::public_identity::paged_generation::*;
use census_domain::model::{ReviewVerdictRecord, StoreIdentity};

mod bounds;
mod corrupt;

fn capture() -> TestResult<CaptureBindingV2> {
    Ok(CaptureBindingV2::new(
        StoreIdentity::parse(&"1".repeat(64))?,
        FormatBindingV2::new(1, 1, 1)?,
        RunBindingV2::Unbound,
        CapturePositionV2::new(9, 7, 12, [3, 4, 5, 6, 7, 8]),
    )?)
}

fn page() -> TestResult<(TableBindingV2, RawPageV2)> {
    let binding = capture()?.table_binding(AuthorityTableV2::IdentityVerdicts)?;
    let value = ReviewVerdictRecord {
        id: "a".into(),
        case_id: "a".into(),
        subject_id: "subject".into(),
        family: "identity".into(),
        kind: "rejected".into(),
        field: "identity".into(),
        value: "unknown".into(),
        accepted: false,
        confidence: 0,
        rationale: "é original bytes".into(),
        reviewer: "fixture".into(),
        observed_at: "2026-10-15".into(),
        member_ids: Vec::new(),
    };
    let mut bytes = serde_json::to_vec(&value)?;
    bytes.extend_from_slice(b"  ");
    let key = crate::keys::observation_key(crate::Table::IdentityVerdicts, "a", 0);
    let row = RawPhysicalRowV2::new(&binding, &key, &bytes)?;
    let page = RawPageV2::new(&binding, RawPagePositionV2::new(0, 0)?, vec![row])?;
    Ok((binding, page))
}

fn raw_plan(bytes: &[u8]) -> TestResult<ObjectPlan> {
    let mut offset = 0usize;
    let ranges = bytes
        .split_inclusive(|byte| *byte == b'\n')
        .map(|line| {
            let range = (offset, line.len());
            offset = offset.checked_add(line.len()).ok_or("offset")?;
            Ok(range)
        })
        .collect::<TestResult<Vec<_>>>()?;
    Ok(ObjectPlan {
        id: object_digest(bytes),
        payload: bytes.to_vec(),
        chunks: bytes
            .chunks(OBJECT_CHUNK_BYTES)
            .map(<[u8]>::to_vec)
            .collect(),
        ranges,
    })
}

fn retain(store: &Store, bytes: &[u8]) -> TestResult<(ObjectPlan, ObjectRefV2)> {
    let plan = raw_plan(bytes)?;
    let rows = pairs(&plan, &plan.id)?;
    let physical = rows
        .iter()
        .try_fold(0u64, |sum, (key, value)| -> TestResult<u64> {
            let key = u64::try_from(key.len())?;
            let value = u64::try_from(value.len())?;
            Ok(sum
                .checked_add(key)
                .and_then(|sum| sum.checked_add(value))
                .ok_or("physical overflow")?)
        })?;
    write(store, &rows)?;
    let object = ObjectRefV2::new(
        DigestV2::parse(&plan.id)?,
        u32::try_from(bytes.len())?,
        u32::try_from(plan.ranges.len())?,
        physical,
    )?;
    Ok((plan, object))
}

fn changed(bytes: &[u8], from: &str, to: &str) -> TestResult<Vec<u8>> {
    let text = std::str::from_utf8(bytes)?;
    check!(text.contains(from));
    Ok(text.replacen(from, to, 1).into_bytes())
}

fn refused<T>(result: StoreResult<T>, expected: &str) -> TestResult {
    match result {
        Err(StoreError::Refused { detail }) => check!(detail.contains(expected), "{detail}"),
        _ => return Err(format!("expected refusal {expected}").into()),
    }
    Ok(())
}

#[test]
fn raw_page_adapter_preserves_original_values_and_full_physical_charge() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (binding, page) = page()?;
    let (plan, object) = retain(&store, &page.encode_jsonl()?)?;
    let before = store.snapshot().sequence();
    let actual = store.snapshot().raw_generation_page(&object, &binding)?;
    check!(eq; actual, page);
    check!(eq; store.snapshot().sequence(), before);
    check!(object.physical_bytes() > u64::from(object.bytes()));
    let (_, replay) = retain(&store, &plan.payload)?;
    check!(eq; replay.physical_bytes(), object.physical_bytes());
    check!(eq; store.snapshot().raw_generation_page(&object, &binding)?, page);
    check!(store.snapshot().sequence() >= before);
    Ok(())
}

#[test]
fn retained_block_table_and_seal_match_offline_decode() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let capture = capture()?;
    let (binding, page) = page()?;
    let (_, page_object) = retain(&store, &page.encode_jsonl()?)?;
    let block = RawPageBlockV2::new(&binding, 0, vec![page.reference(page_object)?])?;
    let (_, block_object) = retain(&store, &block.encode_jsonl()?)?;
    check!(eq; store.snapshot().raw_generation_block(&block_object, &binding)?, block);
    let full = RawTableManifestV2::new(&binding, vec![block.reference(block_object)?])?;
    let (_, full_object) = retain(&store, &full.encode_jsonl()?)?;
    check!(eq; store.snapshot().raw_generation_table(&full_object, &binding)?, full);
    let tables = AuthorityTableV2::ALL
        .into_iter()
        .map(|table| {
            let binding = capture.table_binding(table)?;
            let table = RawTableManifestV2::new(&binding, Vec::new())?;
            let (_, object) = retain(&store, &table.encode_jsonl()?)?;
            check!(eq; store.snapshot().raw_generation_table(&object, &binding)?, table);
            Ok(table.reference(object)?)
        })
        .collect::<TestResult<Vec<_>>>()?;
    let tables: [RawTableReferenceV2; 6] = tables.try_into().map_err(|_| "tables")?;
    let seal = RawGenerationSealV2::new(capture.clone(), tables)?;
    let (_, object) = retain(&store, &seal.encode_jsonl()?)?;
    check!(eq; store.snapshot().raw_generation_seal(&object, &capture)?, seal);
    Ok(())
}

#[test]
fn raw_record_cursor_does_not_collapse_duplicate_payload_fields() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let bytes = b"{\"key\":\"x\",\"at\":\"\",\"payload\":{\"value\":1,\"value\":2}}\n";
    let (plan, _) = retain(&store, bytes)?;
    write(&store, &[alias_pair(&plan.id)?])?;
    let object = read(&store)?.ok_or("object")?;
    let record = object.raw_records().next().ok_or("raw")??;
    check!(eq; record.bytes(), bytes);
    check!(eq; record.sequence(), 0);
    check!(eq; record.offset(), 0);
    check!(eq; record.length(), bytes.len());
    let old = object.records().next().ok_or("old")??;
    check!(eq; old.payload().get("value"), Some(&json!(2)));
    Ok(())
}
