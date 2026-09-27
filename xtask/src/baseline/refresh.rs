use anyhow::{Context, Result};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

use super::NOTE;

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
