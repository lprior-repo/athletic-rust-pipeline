//! Seam enforcement for the census, at module and crate granularity.
//!
//! The census began as one crate kept honest by module seams (`docs/HARDENING-PROGRAM.md` §9), where
//! the compiler seals items (private submodules, visibility) but cannot forbid an edge between
//! top-level modules. It is now being split into lanes, and each split moves an edge out of reach of
//! the module table and into the crate table, so this measurer keeps both:
//!
//! * the module walk reads every production `.rs` file under `crates/census-service/src`, resolves
//!   each `crate::…` reference to its top-level module, and compares the `(from, to)` pair against
//!   [`ALLOWED`];
//! * the crate walk ([`crates`]) reads every workspace package's production source, resolves each
//!   reference to a sibling workspace crate, and compares that pair against [`ALLOWED_CRATES`].
//!
//! A pair outside either table is a violation: the walker names the file and line, prints the full
//! report as JSON on stdout, and fails.
//!
//! Both tables are the ratchet. Adding an edge is a deliberate edit here; deleting a row makes that
//! edge a violation again, because the check fails closed. `(sources, store)` is listed although
//! `ARCHITECTURE.md` calls it a direction violation — `AdapterContext` carries `&Store` today.
//! When the adapters return entity batches instead, delete the row and the walker starts
//! enforcing the narrower graph. `spawn` is the region's task spawner: it reads the clock for its
//! drain deadline and classifies completion through the outcome lattice, and the two modules that
//! own a region (`bootstrap`, `restate_services`) start their tasks through it.
//!
//! Comment lines and test code are out of scope: files named `tests.rs`, files under a `tests/`
//! directory, and the region after the `#[cfg(test)]` that opens a module, because none of them
//! can reach production callers. That judgement is not made here — [`walk`] and [`crates`] ask the
//! scanner's own `is_test_file` and `production_lines`, so a seam check and the size budgets cannot
//! be shown two different production regions.
//!
//! Emits JSON on stdout:
//!
//! ```text
//! {
//!   "modules": ["bests", ...],
//!   "edges": [{"from": "sources", "to": "store", "refs": 24}, ...],
//!   "violations": [{"from": "sources", "to": "report", "file": "...", "line": 283}, ...],
//!   "crates": ["census-store", ...],
//!   "crate_edges": [{"from": "census-service", "to": "census-store", "refs": 96}, ...],
//!   "crate_violations": [{"from": "census-store", "to": "census-report", "file": "...", "line": 4}]
//! }
//! ```
//!
//! The module table is a snapshot of the tree as it is being split: as modules leave the census crate
//! their rows are deleted, and the crate table is what stays behind to enforce the same directions.

use anyhow::{bail, Result};
use serde_json::{json, Value};
use std::collections::BTreeMap;

use crate::json::count;
use crate::paths;
use crate::scan::rules::Rules;
use walk::Reference;

mod crates;
mod parse;
mod walk;

#[cfg(test)]
#[path = "seams/tests.rs"]
mod tests;

/// Allowed edges between the modules still inside `crates/census-service/src`, as `(from, to)`.
///
/// The direction rule is `ARCHITECTURE.md`'s: a lane derives from the rows below it and names no lane
/// back. The table is what a split leaves behind: the adapters, the fetcher, the store, the review lane
/// and the school index have each left for their own crate, so the edges that used to run to them are
/// rows in [`ALLOWED_CRATES`] now (or, where both ends moved together, rules `xtask` no longer needs to
/// state — `net`'s registry lookup lives inside `census-crawl`). The reporting plane left the same way:
/// `bests`, `report` and `workbook` are `census-report`'s modules now, so every row whose far end was
/// one of them is a crate row in [`ALLOWED_CRATES`]. What stays here is the composition root's own
/// graph: the run (`census`), the durable services over it, and the small lanes they share.
///
/// A row is added when a module gains a dependency its design already argued for, never to silence a
/// violation. Edges *inside* another crate are that crate's business and are not walked here.
const ALLOWED: &[(&str, &str)] = &[
    ("bootstrap", "ingress"),
    ("bootstrap", "outcome"),
    ("bootstrap", "restate_services"),
    ("bootstrap", "spawn"),
    ("outcome", "restate_services"),
    ("restate_services", "census"),
    ("restate_services", "outcome"),
    ("restate_services", "spawn"),
    ("spawn", "outcome"),
];

/// Allowed edges between workspace crates, as `(from, to)`.
///
/// The module table one level up: a store writes canonical tables for the lanes that derive them and
/// names no lane back, adapters depend on the domain and not the reverse, and the census binary is
/// the composition root — the only crate that may name every lane. `xtask` sits outside that graph:
/// it is the harness, it drives the census through the library and the services, and nothing depends
/// on it. Rows are added when a crate is extracted, never to silence a violation.
const ALLOWED_CRATES: &[(&str, &str)] = &[
    ("census-crawl", "census-domain"),
    ("census-crawl", "census-store"),
    ("census-review", "census-domain"),
    ("census-review", "census-store"),
    ("census-reconcile", "census-domain"),
    ("census-reconcile", "census-report"),
    ("census-reconcile", "census-store"),
    ("census-report", "census-crawl"),
    ("census-report", "census-domain"),
    ("census-report", "census-review"),
    ("census-report", "census-store"),
    ("census-store", "census-domain"),
    ("census-service", "athleticnet-browser"),
    ("census-service", "census-crawl"),
    ("census-service", "census-reconcile"),
    ("census-service", "census-report"),
    ("census-service", "census-domain"),
    ("census-service", "census-review"),
    ("census-service", "census-store"),
    ("xtask", "census-crawl"),
    ("xtask", "census-domain"),
    ("xtask", "census-report"),
    ("xtask", "census-store"),
    ("xtask", "census-service"),
];

/// Scan the census source tree, print the report, and fail on any disallowed edge.
pub fn run() -> Result<()> {
    let src = paths::census_crate().join("src");
    let rules = Rules::compile()?;
    let (modules, references) = walk::tree(&src, &rules)?;
    let packages = crates::packages()?;
    let (crates, crate_references) = crates::tree(&packages, &rules)?;

    let (edges, violations) = judge(&references, is_allowed);
    let (crate_edges, crate_violations) = judge(&crate_references, is_crate_allowed);

    let modules_bad = violations.len();
    let crates_bad = crate_violations.len();
    let report = json!({
        "modules": modules,
        "edges": edges,
        "violations": violations,
        "crates": crates,
        "crate_edges": crate_edges,
        "crate_violations": crate_violations,
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    if modules_bad > 0 || crates_bad > 0 {
        bail!(
            "{modules_bad} module-seam and {crates_bad} crate-seam violation(s): an edge outside the allowed tables"
        );
    }
    Ok(())
}

/// Group references into edges, and name every reference that sits on a disallowed one.
fn judge(references: &[Reference], allowed: fn(&str, &str) -> bool) -> (Vec<Value>, Vec<Value>) {
    let mut grouped: BTreeMap<(String, String), Vec<&Reference>> = BTreeMap::new();
    for reference in references {
        grouped
            .entry((reference.from.clone(), reference.to.clone()))
            .or_default()
            .push(reference);
    }
    let mut edges = Vec::new();
    let mut violations = Vec::new();
    for ((from, to), group) in &grouped {
        edges.push(json!({"from": from, "to": to, "refs": count(group.len())}));
        if !allowed(from, to) {
            for reference in group {
                violations.push(json!({
                    "from": from,
                    "to": to,
                    "file": reference.file,
                    "line": count(reference.line),
                }));
            }
        }
    }
    (edges, violations)
}

/// Whether `(from, to)` is a declared edge of the census module graph.
fn is_allowed(from: &str, to: &str) -> bool {
    ALLOWED
        .iter()
        .any(|(allowed_from, allowed_to)| *allowed_from == from && *allowed_to == to)
}

/// Whether `(from, to)` is a declared edge of the workspace crate graph.
fn is_crate_allowed(from: &str, to: &str) -> bool {
    ALLOWED_CRATES
        .iter()
        .any(|(allowed_from, allowed_to)| *allowed_from == from && *allowed_to == to)
}
