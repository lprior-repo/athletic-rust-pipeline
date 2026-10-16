use super::ContextPhysicalRef;
use crate::journal::context::{measure_context_physical, stage_context_object};
use crate::journal::{object_chunk_key, object_index_key, object_manifest_key};
use crate::Store;
use fjall::Readable;
use serde_json::{json, Value};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn measured(store: &Store) -> TestResult<super::ContextPhysicalPlan> {
    Ok(measure_context_physical(store, |sink| {
        sink.record(&json!({"row": "one"}))
    })?)
}

fn staged(store: &Store) -> TestResult<ContextPhysicalRef> {
    let plan = measured(store)?;
    stage_context_object(store, |sink| sink.record(&json!({"row": "one"})))?;
    Ok(plan.physical_ref().clone())
}

fn remove(store: &Store, key: &[u8]) -> TestResult {
    let mut batch = store.db.batch();
    batch.remove(&store.journal, key);
    batch.commit()?;
    Ok(())
}

fn corrupt_same_length(store: &Store, key: &[u8]) -> TestResult {
    let mut value = store
        .journal
        .get(key)?
        .ok_or("missing physical row")?
        .to_vec();
    let first = value.first_mut().ok_or("empty physical row")?;
    *first ^= 1;
    let mut batch = store.db.batch();
    batch.insert(&store.journal, key, &value);
    batch.commit()?;
    Ok(())
}

#[test]
fn ready_context_retains_exact_full_inventory_with_zero_incremental_charge() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let initial = measured(&store)?;
    check!(initial.full_bytes() > 0);
    check!(eq; initial.row_count(), 3);
    check!(eq; initial.records(), 1);
    check!(initial.jsonl_bytes() > 0);
    check!(eq; initial.incremental_charge().bytes, initial.full_bytes());
    check!(eq; initial.incremental_charge().roots, 1);
    initial.audit(&store)?;
    stage_context_object(&store, |sink| sink.record(&json!({"row": "one"})))?;
    let ready = measured(&store)?;
    check!(eq; ready.physical_ref(), initial.physical_ref());
    check!(eq; ready.incremental_charge().bytes, 0);
    check!(eq; ready.incremental_charge().roots, 0);
    let total = store
        .db
        .snapshot()
        .prefix(&store.journal, b"j2\0")
        .try_fold(0_u64, |sum, guard| -> TestResult<u64> {
            let (key, value) = guard.into_inner()?;
            Ok(sum
                .checked_add(u64::try_from(
                    key.len()
                        .checked_add(value.len())
                        .ok_or("physical row length overflow")?,
                )?)
                .ok_or("physical sum overflow")?)
        })?;
    check!(eq; ready.full_bytes(), total);
    let encoded = serde_json::to_vec(ready.physical_ref())?;
    check!(encoded.len() <= ready.metadata_bytes_bound());
    let restored: ContextPhysicalRef = serde_json::from_slice(&encoded)?;
    check!(eq; &restored, ready.physical_ref());
    restored.audit(&store)?;
    remove(&store, &object_manifest_key(ready.root())?)?;
    check!(ready.audit(&store).is_err());
    check!(restored.audit_ready(&store).is_err());
    initial.audit(&store)?;
    Ok(())
}

#[test]
fn ready_inventory_rejects_each_missing_or_changed_physical_row() -> TestResult {
    for kind in 0..3 {
        for missing in [true, false] {
            let directory = tempfile::tempdir()?;
            let store = Store::open(directory.path())?;
            let physical = staged(&store)?;
            let key = match kind {
                0 => object_chunk_key(physical.root(), 0)?,
                1 => object_index_key(physical.root(), 0)?,
                _ => object_manifest_key(physical.root())?,
            };
            if missing {
                remove(&store, &key)?;
            } else {
                corrupt_same_length(&store, &key)?;
            }
            if kind == 2 && missing {
                physical.audit(&store)?;
            } else {
                check!(physical.audit(&store).is_err());
            }
        }
    }
    Ok(())
}

#[test]
fn partial_inventory_allows_absent_rows_but_checks_every_retained_fingerprint() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let physical = staged(&store)?;
    remove(&store, &object_manifest_key(physical.root())?)?;
    remove(&store, &object_chunk_key(physical.root(), 0)?)?;
    physical.audit(&store)?;
    let partial = measured(&store)?;
    check!(eq; partial.incremental_charge().bytes, physical.full_bytes());
    check!(eq; partial.incremental_charge().roots, 1);
    corrupt_same_length(&store, &object_index_key(physical.root(), 0)?)?;
    check!(physical.audit(&store).is_err());
    Ok(())
}

#[test]
fn physical_inventory_rejects_extra_ordinals_and_malformed_namespace_keys() -> TestResult {
    for kind in 0..5 {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let physical = staged(&store)?;
        let key = match kind {
            0 => object_chunk_key(physical.root(), 1)?,
            1 => object_index_key(physical.root(), 1)?,
            2 => {
                let mut key = object_manifest_key(physical.root())?;
                key.push(0);
                key
            }
            _ => {
                let mut prefix = crate::journal::object_chunk_prefix(physical.root())?;
                if kind == 3 {
                    prefix.pop().ok_or("chunk prefix is empty")?;
                } else {
                    prefix.push(0);
                }
                prefix
            }
        };
        let mut batch = store.db.batch();
        batch.insert(&store.journal, &key, b"x");
        batch.commit()?;
        check!(physical.audit(&store).is_err());
        check!(measured(&store).is_err());
    }
    Ok(())
}

#[test]
fn physical_inventory_decoder_refuses_unbounded_or_inconsistent_descriptors() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let plan = measured(&store)?;
    let original = serde_json::to_value(plan.physical_ref())?;
    for (field, replacement) in [
        ("root", json!("x".repeat(65))),
        ("full_bytes", json!(0)),
        ("jsonl_bytes", json!(67_108_865_u64)),
        ("records", json!(32_770)),
        ("chunks", json!([])),
        ("pages", json!([])),
    ] {
        let mut raw = original.clone();
        *raw.get_mut(field).ok_or("missing inventory field")? = replacement;
        check!(serde_json::from_value::<ContextPhysicalRef>(raw).is_err());
    }
    for field in ["chunks", "pages"] {
        let mut raw = original.clone();
        let row = raw
            .get(field)
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .ok_or("missing inventory row")?
            .clone();
        *raw.get_mut(field).ok_or("missing inventory array")? = json!(vec![row; 130]);
        check!(serde_json::from_value::<ContextPhysicalRef>(raw).is_err());
    }
    let mut raw = original;
    raw.as_object_mut()
        .ok_or("inventory is not an object")?
        .insert("extra".into(), json!(0));
    check!(serde_json::from_value::<ContextPhysicalRef>(raw).is_err());
    Ok(())
}
