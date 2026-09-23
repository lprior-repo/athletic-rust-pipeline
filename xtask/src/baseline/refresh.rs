//! The refreshed baseline file: the fixed four-key shape, written from current measurements.
//!
//! The writer only ever emits the shape [`crate::baseline`] documents — `note`, `clippy`, `scan`,
//! `structure` — because `tools/quality-baseline.json` is read by tracked tooling and by the ratchet
//! itself. The scan's `crates` and `structure` objects are copied through unchanged rather than
//! re-serialised field by field, so a new metric the scan starts reporting lands in the file without
//! a second list of names to keep in step.

use anyhow::{Context, Result};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

use super::NOTE;

/// The refreshed baseline file, with current measurements.
pub(super) fn baseline(clippy: BTreeMap<String, u64>, scan: &Value) -> Result<String> {
    let crates = scan
        .get("crates")
        .cloned()
        .context("the scan report has no `crates` object")?;
    let structure = scan
        .get("structure")
        .cloned()
        .context("the scan report has no `structure` object")?;
    let mut baseline: Map<String, Value> = Map::new();
    baseline.insert("note".to_string(), Value::String(NOTE.to_string()));
    baseline.insert(
        "clippy".to_string(),
        Value::Object(
            clippy
                .into_iter()
                .map(|(key, value)| (key, Value::from(value)))
                .collect(),
        ),
    );
    baseline.insert("scan".to_string(), crates);
    baseline.insert("structure".to_string(), structure);
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&Value::Object(baseline))?
    ))
}
