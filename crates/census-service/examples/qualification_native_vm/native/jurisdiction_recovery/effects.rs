use super::super::super::{artifacts, GUEST};
use anyhow::{ensure, Context, Result};
use census_store::{NativeEffectCheckpoint, Store};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;

pub(super) fn read(store: &Store) -> Result<Value> {
    let boundary = artifacts::json(&Path::new(GUEST).join("jurisdiction-recovery-boundary.json"))?;
    let previous: NativeEffectCheckpoint = serde_json::from_value(
        boundary
            .pointer("/boundary/witness/native_source_boundary/marker/content/acknowledged_effects")
            .context("pre-reset owning-store acknowledged effect checkpoint absent")?
            .clone(),
    )?;
    let current = store.native_effect_checkpoint()?;
    ensure!(
        previous.schema == 1 && current.schema == 1 && current.sequence >= previous.sequence,
        "original acknowledged checkpoint schema or store sequence regressed"
    );
    ensure!(
        !previous.rows.is_empty() && !previous.receipts.is_empty(),
        "reboot qualification has no acknowledged physical source effects to conserve"
    );
    conserve_rows(&previous, &current)?;
    conserve_receipts(&previous, &current)?;
    Ok(json!({
        "before":previous,"after":current,"preserved":true,
        "scope":"exact raw source observation occurrence keys and persisted value digests plus acknowledged full receipts; new honest effects permitted; derived projections may publish normally"
    }))
}

fn conserve_rows(
    previous: &NativeEffectCheckpoint,
    current: &NativeEffectCheckpoint,
) -> Result<()> {
    ensure!(
        previous.rows.len() <= 100_000 && current.rows.len() <= 100_000,
        "effect row checkpoint bound exceeded"
    );
    let mut retained = HashMap::new();
    retained.try_reserve(current.rows.len())?;
    for row in &current.rows {
        ensure!(
            retained
                .insert((row.table.file(), row.key.as_slice()), row.digest.as_str())
                .is_none(),
            "cold checkpoint has duplicate physical occurrence keys"
        );
    }
    let mut original = HashMap::new();
    original.try_reserve(previous.rows.len())?;
    for row in &previous.rows {
        let key = (row.table.file(), row.key.as_slice());
        ensure!(
            original.insert(key, ()).is_none(),
            "original checkpoint has duplicate physical occurrence keys"
        );
        ensure!(
            retained.get(&key).copied() == Some(row.digest.as_str()),
            "acknowledged physical {} occurrence changed or disappeared across reboot",
            row.table.file()
        );
    }
    Ok(())
}

fn conserve_receipts(
    previous: &NativeEffectCheckpoint,
    current: &NativeEffectCheckpoint,
) -> Result<()> {
    ensure!(
        previous.receipts.len() <= 100_000 && current.receipts.len() <= 100_000,
        "effect receipt checkpoint bound exceeded"
    );
    let mut retained = HashMap::new();
    retained.try_reserve(current.receipts.len())?;
    for receipt in &current.receipts {
        ensure!(
            retained
                .insert(receipt.operation.as_str(), receipt)
                .is_none(),
            "cold checkpoint has duplicate receipt operations"
        );
    }
    let mut original = HashMap::new();
    original.try_reserve(previous.receipts.len())?;
    for receipt in &previous.receipts {
        ensure!(
            original.insert(receipt.operation.as_str(), ()).is_none(),
            "original checkpoint has duplicate receipt operations"
        );
        ensure!(
            retained.get(receipt.operation.as_str()).copied() == Some(receipt),
            "acknowledged receipt {} changed or disappeared across reboot",
            receipt.operation
        );
    }
    Ok(())
}
