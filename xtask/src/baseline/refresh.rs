use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;

use super::NOTE;

#[derive(serde::Serialize)]
struct Baseline<'a> {
    clippy: BTreeMap<String, u64>,
    note: &'static str,
    scan: &'a Value,
    structure: &'a Value,
}

pub(super) fn baseline(clippy: BTreeMap<String, u64>, scan: &Value) -> Result<String> {
    let crates = scan
        .get("crates")
        .context("the scan report has no `crates` object")?;
    let structure = scan
        .get("structure")
        .context("the scan report has no `structure` object")?;
    let baseline = Baseline {
        clippy,
        note: NOTE,
        scan: crates,
        structure,
    };
    Ok(format!("{}\n", serde_json::to_string_pretty(&baseline)?))
}
