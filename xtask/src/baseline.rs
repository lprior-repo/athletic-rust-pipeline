use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

use crate::json::truthy;
use crate::paths;
use compare::raises;
use measure::{clippy_counts, read_json};

mod compare;
mod measure;
mod refresh;

const NOTE: &str = "Debt baseline for tools/gate.sh. Numbers may only shrink; refresh with tools/gate.sh --update-baseline after a burndown.";

const STRUCTURE_METRICS: [&str; 1] = ["functions_over_60_lines"];

const CONTEXT_METRICS: [&str; 2] = ["files", "production_lines"];

const OVERSIZED_FILES: &str = "files_over_300_lines";

const FUNCTION_SITES: &str = "functions_over_60_sites";

pub fn update(
    baseline: &Path,
    clippy_tsv: &Path,
    scan_json: &Path,
    allow_increase: bool,
) -> Result<()> {
    let clippy = clippy_counts(clippy_tsv)?;
    let scan = read_json(scan_json)?;
    crate::scan::strict::enforce_report(&scan)?;
    let old = if baseline.exists() {
        read_json(baseline)?
    } else {
        Value::Object(Map::new())
    };
    refuse_increases(&clippy, &scan, &old, allow_increase)?;

    let written = refresh::baseline(clippy, &scan)?;
    fs::write(baseline, written)
        .with_context(|| format!("writing {}", paths::relative(baseline)))?;
    println!("baseline updated: {}", paths::relative(baseline));
    Ok(())
}

fn refuse_increases(
    clippy: &std::collections::BTreeMap<String, u64>,
    scan: &Value,
    old: &Value,
    allow_increase: bool,
) -> Result<()> {
    if allow_increase || !truthy(old) {
        return Ok(());
    }
    let raised = raises(clippy, scan, old)?;
    if !raised.is_empty() {
        println!("refusing to raise the baseline without --allow-increase:");
        for item in &raised {
            println!("  {item}");
        }
        bail!("the debt baseline was not written");
    }
    Ok(())
}

pub fn ratchet(baseline: &Path, clippy_tsv: &Path, scan_json: &Path) -> Result<()> {
    let known = read_json(baseline)?;
    let clippy = clippy_counts(clippy_tsv)?;
    let scan = read_json(scan_json)?;
    crate::scan::strict::enforce_report(&scan)?;
    let mut failures: Vec<String> = Vec::new();

    compare::clippy(&known, &clippy, &mut failures)?;
    compare::scan(&scan, &known, &mut failures)?;
    compare::structure(&scan, &known, &mut failures)?;

    if failures.is_empty() {
        println!("ratchet: no metric grew");
        return Ok(());
    }
    println!("ratchet failures (debt grew):");
    for item in &failures {
        println!("  {item}");
    }
    bail!(
        "debt grew in {} metric(s); see the failures above",
        failures.len()
    )
}
