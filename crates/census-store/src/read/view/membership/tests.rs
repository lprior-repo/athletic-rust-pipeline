use super::*;
use crate::journal::{chunk_key, chunk_value, manifest_value};
use crate::{Store, StoreError, MAX_JOURNAL_VALUE_BYTES};
use serde_json::json;

mod admission;
mod bounds;
mod closure;
mod descriptors;
mod fingerprints;
mod provisional;
mod snapshot;
mod wire;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const PHASE: &str = "membership_fixture_v1";
const TEXT_REFUSAL: &str = "journal namespace and logical keys must be nonempty control-free text";

fn namespace() -> StoreResult<JournalNamespace<'static>> {
    JournalNamespace::new(PHASE, "row/")
}

fn budget(rows: u64) -> ReadBudget {
    ReadBudget {
        rows,
        bytes: 8 * 1024 * 1024,
        record_bytes: MAX_JOURNAL_VALUE_BYTES,
    }
}

fn put(store: &Store, phase: &str, key: &str, revision: u64) -> StoreResult<()> {
    let mut batch = store.write_batch();
    batch.journal_done(phase, key, &json!({"revision": revision}))?;
    batch.commit()?;
    Ok(())
}

fn raw(store: &Store, key: &str, value: &[u8]) -> TestResult {
    store
        .journal
        .insert(Store::journal_key(PHASE, key), value)?;
    Ok(())
}

fn roots<'s>(view: &StoreSnapshot<'s>) -> StoreResult<JournalNamespaceInventory<'s>> {
    view.for_each_journal_root_bounded(namespace()?, budget(100), |_| Ok(()))
}

fn descriptor<T>(result: StoreResult<T>, lookup: &str, reason: &str) -> TestResult {
    check!(
        matches!(result, Err(StoreError::JournalDescriptor { phase, key, detail })
        if phase == PHASE && key == lookup && detail == reason)
    );
    Ok(())
}

fn text_refusal<T>(result: StoreResult<T>) -> TestResult {
    check!(matches!(result, Err(StoreError::Refused { detail }) if detail == TEXT_REFUSAL));
    Ok(())
}

fn encoded_bytes(key: &[u8], value: &[u8]) -> StoreResult<u64> {
    let size = key
        .len()
        .checked_add(value.len())
        .ok_or(StoreError::CounterOverflow)?;
    u64::try_from(size).map_err(|_| StoreError::CounterOverflow)
}

fn ready_alias() -> Result<Vec<u8>, serde_json::Error> {
    serde_json::to_vec(&json!({
        "type": "journal-alias", "version": 2, "state": "ready", "object": "a".repeat(64),
    }))
}
