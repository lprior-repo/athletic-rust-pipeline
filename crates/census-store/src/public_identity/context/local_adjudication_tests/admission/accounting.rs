use super::*;
use fjall::Readable;
use serde_json::Value;

const HEADER: &str = "manual-context-admission-v1\0header";
const MAX_BYTES: u64 = 128 * 1024 * 1024;

enum Alteration {
    ZeroCharges,
    FillCapacity,
}

#[test]
fn undercharged_retry_refuses_before_any_context_write() -> TestResult {
    let fixture = merged()?;
    let request = request(&fixture)?;
    let head = fixture.writer.head(&fixture.store)?;
    let operation = "admission-undercharge";
    fixture.writer.reserve_local_adjudication(
        &fixture.store,
        operation,
        request.clone(),
        fixture.store.snapshot().revision(),
        &head,
    )?;
    alter(&fixture.store, operation, Alteration::ZeroCharges)?;
    let before = fixture.store.snapshot().sequence();
    assert!(fixture
        .writer
        .prepare_local_adjudication(
            &fixture.store,
            operation,
            request,
            fixture.store.snapshot().revision(),
            &head
        )
        .is_err());
    assert_eq!(before, fixture.store.snapshot().sequence());
    Ok(())
}

#[test]
fn exhausted_admission_refuses_new_root_before_writes_but_original_retry_completes() -> TestResult {
    let fixture = merged()?;
    let request = request(&fixture)?;
    let head = fixture.writer.head(&fixture.store)?;
    let operation = "admission-full-original";
    let reservation = fixture.writer.reserve_local_adjudication(
        &fixture.store,
        operation,
        request.clone(),
        fixture.store.snapshot().revision(),
        &head,
    )?;
    let original = serde_json::to_vec(reservation.prepared.record())?;
    drop(reservation);
    alter(&fixture.store, operation, Alteration::FillCapacity)?;
    let before = fixture.store.snapshot().sequence();
    assert!(fixture
        .writer
        .prepare_local_adjudication(
            &fixture.store,
            "admission-full-new",
            request.clone(),
            fixture.store.snapshot().revision(),
            &head
        )
        .is_err());
    assert_eq!(before, fixture.store.snapshot().sequence());
    let retry = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        operation,
        request,
        fixture.store.snapshot().revision(),
        &head,
    )?;
    assert_eq!(original, serde_json::to_vec(retry.record())?);
    Ok(())
}

fn alter(store: &crate::Store, operation: &str, alteration: Alteration) -> TestResult {
    let key = format!(
        "manual-context-admission-v1\0reservation\0{}",
        crate::journal::object_digest(operation.as_bytes())
    );
    let snapshot = store.db.snapshot();
    let original = snapshot.get(&store.journal, &key)?.ok_or("reservation")?;
    let raw_header = snapshot.get(&store.journal, HEADER)?.ok_or("header")?;
    let mut row: Value = serde_json::from_slice(&original)?;
    let mut header: Value = serde_json::from_slice(&raw_header)?;
    let old = contribution(&key, &row)?;
    let base = header["bytes"]
        .as_u64()
        .ok_or("header bytes")?
        .checked_sub(old)
        .ok_or("base")?;
    let old_roots = sum(&row, "roots")?;
    match alteration {
        Alteration::ZeroCharges => {
            for charge in row["charges"].as_array_mut().ok_or("charges")? {
                charge["bytes"] = 0.into();
                charge["roots"] = 0.into();
            }
        }
        Alteration::FillCapacity => fill(&key, &mut row, base)?,
    }
    header["bytes"] = base
        .checked_add(contribution(&key, &row)?)
        .ok_or("header bytes")?
        .into();
    header["roots"] = header["roots"]
        .as_u64()
        .ok_or("header roots")?
        .checked_sub(old_roots)
        .and_then(|value| value.checked_add(sum(&row, "roots").ok()?))
        .ok_or("roots")?
        .into();
    let mut batch = store.db.batch();
    batch.insert(&store.journal, key, serde_json::to_vec(&row)?);
    batch.insert(&store.journal, HEADER, serde_json::to_vec(&header)?);
    batch
        .durability(Some(fjall::PersistMode::SyncData))
        .commit()?;
    Ok(())
}

fn fill(key: &str, row: &mut Value, base: u64) -> TestResult {
    row["charges"][1]["bytes"] = MAX_BYTES.into();
    for _ in 0..2 {
        let total = base.checked_add(contribution(key, row)?).ok_or("total")?;
        let excess = total.checked_sub(MAX_BYTES).ok_or("excess")?;
        row["charges"][1]["bytes"] = row["charges"][1]["bytes"]
            .as_u64()
            .ok_or("bytes")?
            .checked_sub(excess)
            .ok_or("allowance")?
            .into();
    }
    assert_eq!(base.checked_add(contribution(key, row)?), Some(MAX_BYTES));
    Ok(())
}

fn contribution(key: &str, row: &Value) -> TestResult<u64> {
    let raw = serde_json::to_vec(row)?;
    let size = u64::try_from(key.len().checked_add(raw.len()).ok_or("size")?)?;
    Ok(size.checked_add(sum(row, "bytes")?).ok_or("contribution")?)
}

fn sum(row: &Value, field: &str) -> TestResult<u64> {
    row["charges"]
        .as_array()
        .ok_or("charges")?
        .iter()
        .try_fold(0u64, |sum, charge| {
            Ok(sum
                .checked_add(charge[field].as_u64().ok_or("charge")?)
                .ok_or("sum")?)
        })
}

pub(super) fn saturate(store: &crate::Store, operation: &str) -> TestResult {
    alter(store, operation, Alteration::FillCapacity)
}
