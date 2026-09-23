//! Seam enforcement for the census, at module and crate granularity.
//!
//! The census began as one crate kept honest by module seams (`docs/HARDENING-PROGRAM.md` §9), where
//! the compiler seals items (private submodules, visibility) but cannot forbid an edge between
//! top-level modules. It is now being split into lanes, and each split moves an edge out of reach of
//! the module table and into the crate table, so this measurer keeps both:
//!
//! * the module walk reads every production `.rs` file under `crates/midwest-census/src`, resolves
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
//!   "crate_edges": [{"from": "midwest-census", "to": "census-store", "refs": 96}, ...],
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

/// Allowed edges between top-level modules of `crates/midwest-census/src`, as `(from, to)`.
///
/// The direction rule is `ARCHITECTURE.md`'s: adapters and workflows depend on domain types and
/// on the store, never the reverse; `net` and `school_index` are leaves, with the one recorded
/// exception below for the transport the registry declares; `store` and `report` may not reach into
/// `net` (the clock lives in `clock`).
const ALLOWED: &[(&str, &str)] = &[
    ("bests", "report"),
    ("bests", "store"),
    ("bootstrap", "clock"),
    ("bootstrap", "outcome"),
    ("bootstrap", "restate_services"),
    ("bootstrap", "spawn"),
    ("bootstrap", "store"),
    ("census", "clock"),
    ("census", "net"),
    ("census", "report"),
    ("census", "school_index"),
    ("census", "sources"),
    ("census", "store"),
    // The provenance gate is a fetching adapter over the fragment CSVs: it re-derives every shipped
    // coach row from the page that row cites, through the same polite fetcher the sources use (three
    // passes, one shared on-disk cache) and stamps its manifest with that fetcher's clock. `net` is
    // the one module it needs beyond the standard library and the CSV reader.
    ("coachverify", "net"),
    // The verified fragment is published the way every other derived artifact is: through the store's
    // atomic-rename writer and its CSV failure type, so a reader never sees a half-written state file
    // and one publication bug has one implementation. The lane reads no canonical row and writes none:
    // the edge is the writer plumbing, exactly as it is for `index` and `bests`.
    ("coachverify", "store"),
    ("index", "report"),
    ("index", "store"),
    // §29-§31: the derived indexes are the workbook's other reader. The queues, coverage and
    // snapshot rows the store keeps are the same findings the sheets print, so the index module
    // composes the workbook's retained-record families rather than re-deriving them, and a store
    // reader and a workbook reader cannot be shown different findings.
    ("index", "workbook"),
    // The review lane is a reader of retained findings and a writer of verdicts: it asks about the
    // cases the store kept (`ReviewCases`) and the canonical rows behind them, and writes back only
    // `IdentityVerdicts` and the cases' own state. It never edits a canonical row, and `store` never
    // names `identity`, so the edge runs the direction ARCHITECTURE.md sets — a lane over the store,
    // not the store over a lane.
    ("identity", "store"),
    ("net", "clock"),
    // The recorded exception to `net` being a leaf: which transport carries a host — HTTP or the
    // browser lane — is the source registry's declaration, not the caller's request, so the executor
    // asks the registry rather than inferring a transport from the hostname. The dependency is one
    // lookup of a declared fact (`transport_for_host`), never a fetch and never a descriptor walk.
    ("net", "sources"),
    ("outcome", "restate_services"),
    ("report", "clock"),
    ("report", "store"),
    ("restate_services", "bests"),
    ("restate_services", "census"),
    ("restate_services", "clock"),
    // ARCHITECTURE.md §1: the batch path and the durable path share the adapters, the store and the
    // reports, and differ only in who owns the journal. The workflow layer therefore names the
    // adapter error type it classifies into the durable retry policy (`jobs::collect_error`) and the
    // polite fetcher the jurisdiction object holds for the whole process, which is also what keeps
    // one remote origin drawing from one admission budget (§3).
    ("restate_services", "net"),
    ("restate_services", "sources"),
    ("restate_services", "outcome"),
    ("restate_services", "report"),
    ("restate_services", "spawn"),
    ("restate_services", "store"),
    ("restate_services", "workbook"),
    ("sources", "net"),
    ("sources", "school_index"),
    ("sources", "store"),
    ("spawn", "clock"),
    ("spawn", "outcome"),
    ("store", "clock"),
    // The athlete key (`school, normalized name, cohort`) has one definition, in the identity lane's
    // flags, and the conflict queue groups by that same function rather than stating a second copy of
    // the key that could drift from the flags the review lane states to a model. Nothing else of the
    // identity lane is named: the queue reads the key and the store's rows.
    ("workbook", "identity"),
    ("workbook", "bests"),
    ("workbook", "report"),
    // The meta sheets render the adapter surface itself — slug, transport, declared capabilities
    // and the per-origin request cost — so the workbook reads the registry table as data. It never
    // calls an adapter and never fetches.
    ("workbook", "sources"),
    ("workbook", "store"),
];

/// Allowed edges between workspace crates, as `(from, to)`.
///
/// The module table one level up: a store writes canonical tables for the lanes that derive them and
/// names no lane back, adapters depend on the domain and not the reverse, and the census binary is
/// the composition root — the only crate that may name every lane. `xtask` sits outside that graph:
/// it is the harness, it drives the census through the library and the services, and nothing depends
/// on it. Rows are added when a crate is extracted, never to silence a violation.
const ALLOWED_CRATES: &[(&str, &str)] = &[
    // The original tree, carried over: the root binary drives a persistent Chromium session through
    // the browser crate, which knows nothing about the census.
    ("athletic-rust-pipeline", "athleticnet-browser"),
    // The bottom of the graph: types and their rules, no store, no network, no runtime.
    ("census-review", "census-domain"),
    // The review lane reads retained cases and writes verdicts through the store that owns both
    // tables; it never opens Fjall itself.
    ("census-review", "census-store"),
    ("census-store", "census-domain"),
    ("midwest-census", "census-domain"),
    ("midwest-census", "census-review"),
    ("midwest-census", "census-store"),
    // The harness reads the store for its status verb and drives the census services through their
    // ingress clients, so it names both.
    ("xtask", "census-domain"),
    ("xtask", "census-store"),
    ("xtask", "midwest-census"),
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
