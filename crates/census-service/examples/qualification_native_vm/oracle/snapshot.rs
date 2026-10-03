use super::super::{artifacts, drain, process, GUEST};
use super::{capture, equal_rows, payload_digest};
use anyhow::{ensure, Context, Result};
use census_domain::model::CanonicalAthlete;
use census_store::{Receipt, Store, StoreError, StoreSnapshot, Table};
use serde_json::{json, Value};
use std::path::Path;

pub(in super::super) fn read() -> Result<Value> {
    let root = Path::new(GUEST);
    let drain_certificate = require_offline_owner(root)?;
    let payload = artifacts::json(&root.join("request.json"))?;
    let acknowledgement = artifacts::json(&root.join("ingest-first.json"))?;
    let expected = payload
        .get("rows")
        .and_then(Value::as_array)
        .context("expected rows absent")?;
    ensure!(
        !expected.is_empty() && expected.len() <= 100,
        "require 1..100 genuine Class-of-2027 rows"
    );
    let store = Store::open(root.join("store"))?;
    let receipt = acknowledged_receipt(&store, &payload, &acknowledgement, expected.len())?;
    let snapshot = store.snapshot();
    let physical = physical_rows(&snapshot, expected)?;
    equal_rows(expected, &physical)?;
    let captures = ["index", "roster"]
        .into_iter()
        .map(|name| capture(root, name))
        .collect::<Result<Vec<_>>>()?;
    let result = json!({"physical_rows":physical,"tables_digest":snapshot.tables_digest(&[Table::Athletes])?,"receipt":receipt,"captures":captures,"acknowledgement":acknowledgement,"manifest":artifacts::json(&root.join("manifest.json"))?,"deployment":artifacts::json(&root.join("deployment.json"))?,"single_owner_offline":true,"drain_certificate":drain_certificate});
    drop(snapshot);
    drop(store);
    artifacts::publish(&root.join("physical-snapshot.json"), &result)?;
    Ok(result)
}

pub(in super::super) fn drain_certificate() -> Result<Value> {
    require_offline_owner(Path::new(GUEST))
}

fn require_offline_owner(root: &Path) -> Result<Value> {
    let pid = process::command(
        root,
        "snapshot-owner-check",
        std::process::Command::new("/usr/bin/systemctl").args([
            "show",
            "--property=MainPID",
            "--value",
            "qualification.service",
        ]),
        100,
    )?;
    ensure!(
        pid.trim() == "0",
        "offline reader refuses a live service owner"
    );
    let result = process::command(
        root,
        "snapshot-drain-check",
        std::process::Command::new("/usr/bin/systemctl").args([
            "show",
            "--property=Result",
            "--value",
            "qualification.service",
        ]),
        100,
    )?;
    ensure!(
        result.trim() == "success",
        "guest service orderly drain failed: {result}"
    );
    let boot = std::fs::read_to_string("/proc/sys/kernel/random/boot_id")?;
    drain::verify(root, boot.trim())
}

fn acknowledged_receipt(
    store: &Store,
    payload: &Value,
    acknowledgement: &Value,
    expected: usize,
) -> Result<Receipt> {
    let operation = payload
        .get("operation_id")
        .and_then(Value::as_str)
        .context("operation absent")?;
    let receipt = store
        .receipt(operation)?
        .context("acknowledged receipt absent after fault")?;
    ensure!(
        receipt.digest == payload_digest(payload)?,
        "durable receipt content digest differs"
    );
    ensure!(
        receipt.appended == u64::try_from(expected)?,
        "receipt physical row count differs"
    );
    ensure!(
        Some(&json!(receipt.at))
            == acknowledgement
                .get("reply")
                .and_then(|reply| reply.get("last_appended_at")),
        "receipt original date differs from host-acknowledged effect"
    );
    Ok(receipt)
}

fn physical_rows(snapshot: &StoreSnapshot<'_>, expected: &[Value]) -> Result<Vec<Value>> {
    let ids = expected
        .iter()
        .map(|row| {
            row.get("id")
                .and_then(Value::as_str)
                .context("acknowledged athlete ID absent")
        })
        .collect::<Result<std::collections::HashSet<_>>>()?;
    ensure!(
        ids.len() == expected.len(),
        "acknowledged fixture athlete IDs are not distinct"
    );
    let mut physical = Vec::new();
    physical
        .try_reserve(100)
        .context("reserving bounded physical observations")?;
    let mut visited = 0_usize;
    snapshot.for_each_observation::<CanonicalAthlete>(Table::Athletes, |row| {
        visited = visited
            .checked_add(1)
            .ok_or_else(|| StoreError::Invariant {
                detail: "physical observation count overflow".into(),
            })?;
        if visited > 10_000 {
            return Err(StoreError::Invariant {
                detail: "VM qualification physical scan budget exceeded".into(),
            });
        }
        if !ids.contains(row.id.as_str()) {
            return Ok(());
        }
        if physical.len() >= 100 {
            return Err(StoreError::Invariant {
                detail: "VM qualification physical row budget exceeded".into(),
            });
        }
        physical.push(
            serde_json::to_value(row).map_err(|error| StoreError::Invariant {
                detail: error.to_string(),
            })?,
        );
        Ok(())
    })?;
    Ok(physical)
}
