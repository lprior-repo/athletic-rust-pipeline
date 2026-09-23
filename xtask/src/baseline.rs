//! The debt baseline: rewrite it from current measurements, and ratchet the measurements against it.
//!
//! The baseline is a ratchet. `update` refuses to raise any number unless `--allow-increase` says the
//! increase is deliberate; `ratchet` fails the gate when any metric grew, and prints every change as
//! `[DOWN]` or `[UP]` so burndown is visible on every run.
//!
//! The JSON shape is fixed: `note`, `clippy` (keyed `crate\tlint`), `scan` (keyed by crate) and
//! `structure`. `tools/quality-baseline.json` is read by tracked tooling, so the keys and their
//! nesting do not move. The constants below name that shape; [`refresh`] writes it, [`compare`] reads
//! a measurement against it, and [`measure`] turns the gate's two measurement files into values.

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

/// Written into every refreshed baseline, exactly as the deleted script wrote it.
const NOTE: &str = "Debt baseline for tools/gate.sh. Numbers may only shrink; refresh with tools/gate.sh --update-baseline after a burndown.";

/// The structure metrics the ratchet fails on: budget overruns a fix can remove.
const STRUCTURE_METRICS: [&str; 1] = ["functions_over_60_lines"];

/// The structure metric that is reported but never fatal.
///
/// `functions_over_25_logical_lines` is a target, not a budget: it counts every function past 25
/// logical lines, this tree has more than five hundred of them, and adding one ordinary function
/// pushes it up. A metric that must grow whenever the codebase grows cannot be a ratchet — a gate
/// that fails on it fails on every feature commit and gets rubber-stamped. It prints as `[UP]` so the
/// drift stays visible, and the hard one-page budget in `STRUCTURE_METRICS` is what the gate holds.
const SOFT_STRUCTURE_METRICS: [&str; 1] = ["functions_over_25_logical_lines"];

/// Scan metrics that describe the tree rather than its debt. They grow with every line of new code,
/// so failing on them would make the gate a code freeze instead of a quality gate: printed, never
/// fatal.
///
/// Everything else ratchets, including a metric the scan starts reporting later: the forbidden
/// constructs (`scan.rs` holds those names) are each held at zero for new code, so an increase is new
/// debt, and an unclassified name fails once until the baseline records what it is.
const CONTEXT_METRICS: [&str; 2] = ["files", "production_lines"];

/// The structure key that holds one display entry per oversized file rather than a number.
const OVERSIZED_FILES: &str = "files_over_300_lines";

/// The structure key that names each function over the physical-line budget (`evidence`, not a
/// metric: the count above is what the ratchet compares).
const FUNCTION_SITES: &str = "functions_over_60_sites";

/// Rewrite `baseline` from the current clippy and scan measurements.
///
/// Refuses to raise a number without `allow_increase`: a burndown is the only legitimate reason for
/// the baseline to move down.
pub fn update(
    baseline: &Path,
    clippy_tsv: &Path,
    scan_json: &Path,
    allow_increase: bool,
) -> Result<()> {
    let clippy = clippy_counts(clippy_tsv)?;
    let scan = read_json(scan_json)?;
    let old = if baseline.exists() {
        read_json(baseline)?
    } else {
        Value::Object(Map::new())
    };

    if !allow_increase && truthy(&old) {
        let raised = raises(&clippy, &scan, &old)?;
        if !raised.is_empty() {
            println!("refusing to raise the baseline without --allow-increase:");
            for item in &raised {
                println!("  {item}");
            }
            bail!("the debt baseline was not written");
        }
    }

    let written = refresh::baseline(clippy, &scan)?;
    fs::write(baseline, written)
        .with_context(|| format!("writing {}", paths::relative(baseline)))?;
    println!("baseline updated: {}", paths::relative(baseline));
    Ok(())
}

/// Compare current measurements against `baseline`, failing when any metric grew.
pub fn ratchet(baseline: &Path, clippy_tsv: &Path, scan_json: &Path) -> Result<()> {
    let known = read_json(baseline)?;
    let clippy = clippy_counts(clippy_tsv)?;
    let scan = read_json(scan_json)?;
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
