use anyhow::{ensure, Context, Result};
use census_store::Store;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

use super::artifacts::write_json;

pub const PHASE: &str = "teams_source_attempts_v1";

#[derive(Serialize)]
pub struct Snapshot {
    pub phase: &'static str,
    pub sequence: u64,
    pub entries: BTreeMap<String, Value>,
}

pub fn capture(root: &Path, cleanup: &Value) -> Result<Snapshot> {
    capture_named(root, cleanup, "source-ledger-cold.json")
}

pub fn capture_named(root: &Path, cleanup: &Value, name: &str) -> Result<Snapshot> {
    ensure!(
        cleanup
            .get("endpoint_term_reap")
            .and_then(|value| value.get("success"))
            .and_then(Value::as_bool)
            == Some(true),
        "cold inspection requires reaped endpoint"
    );
    ensure!(
        cleanup
            .get("drain_certificate")
            .and_then(|value| value.get("balanced"))
            .and_then(Value::as_bool)
            == Some(true),
        "cold inspection requires measured drain certificate"
    );
    let store = Store::open(root.join("store"))
        .context("opening owned store after ordered endpoint exit")?;
    let keys = store.journal_keys(PHASE)?;
    ensure!(
        keys.len() <= 128,
        "source ledger exceeds qualification slot-read budget"
    );
    let view = store.snapshot();
    let sequence = view.sequence();
    let entries = keys
        .into_iter()
        .map(|key| -> Result<_> {
            let payload = view
                .journal_payload(PHASE, &key)?
                .context("enumerated source journal key has no payload")?;
            Ok((key, payload))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    drop(view);
    store.flush()?;
    drop(store);
    let snapshot = Snapshot {
        phase: PHASE,
        sequence,
        entries,
    };
    write_json(
        &root.join(name),
        &json!({"inspection":"after orderly TERM, drain and endpoint reap; never a second live store owner", "snapshot":snapshot,
            "slot_byte_immutability_before_after":"UNPROVEN: final cold snapshot is not two restart-separated byte snapshots", "cleanup":cleanup}),
    )?;
    Ok(snapshot)
}
