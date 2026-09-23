//! Module-seam enforcement for the census crate.
//!
//! `docs/HARDENING-PROGRAM.md` §9 keeps the census a single crate with module seams, so the
//! compiler seals items (private submodules, visibility) but cannot forbid an edge between
//! top-level modules. This measurer reads every production `.rs` file under
//! `crates/midwest-census/src`, resolves each `crate::…` reference to its top-level module, and
//! compares the `(from, to)` pair against [`ALLOWED`]. A pair outside the table is a violation:
//! the walker names the file and line, prints the full report as JSON on stdout, and fails.
//!
//! The table is the ratchet. Adding an edge is a deliberate edit here; deleting a row makes that
//! edge a violation again, because the check fails closed. `(sources, store)` is listed although
//! `ARCHITECTURE.md` calls it a direction violation — `AdapterContext` carries `&Store` today.
//! When the adapters return entity batches instead, delete the row and the walker starts
//! enforcing the narrower graph. `spawn` is the region's task spawner: it reads the clock for its
//! drain deadline and classifies completion through the outcome lattice, and the two modules that
//! own a region (`bootstrap`, `restate_services`) start their tasks through it.
//!
//! Comment lines and test code are out of scope: files named `tests.rs`, files under a `tests/`
//! directory, and the region after the `#[cfg(test)]` that opens a module, because none of them
//! can reach production callers. That judgement is not made here — [`walk`] asks the scanner's own
//! `is_test_file` and `production_lines`, so the seam check and the size budgets cannot be shown
//! two different production regions.
//!
//! Emits JSON on stdout:
//!
//! ```text
//! {
//!   "modules": ["bests", ...],
//!   "edges": [{"from": "sources", "to": "store", "refs": 24}, ...],
//!   "violations": [{"from": "sources", "to": "report", "file": "...", "line": 283}, ...]
//! }
//! ```

use anyhow::{bail, Result};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

use crate::json::count;
use crate::paths;
use crate::scan::rules::Rules;
use walk::Reference;

mod parse;
mod walk;

#[cfg(test)]
#[path = "seams/tests.rs"]
mod tests;

/// Allowed edges between top-level modules of `crates/midwest-census/src`, as `(from, to)`.
///
/// The direction rule is `ARCHITECTURE.md`'s: adapters and workflows depend on domain types and
/// on the store, never the reverse; `net` and `school_index` are leaves; `store` and `report` may
/// not reach into `net` (the clock lives in `clock`).
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
    ("workbook", "bests"),
    ("workbook", "report"),
    // The meta sheets render the adapter surface itself — slug, transport, declared capabilities
    // and the per-origin request cost — so the workbook reads the registry table as data. It never
    // calls an adapter and never fetches.
    ("workbook", "sources"),
    ("workbook", "store"),
];

/// Scan the census source tree, print the report, and fail on any disallowed edge.
pub fn run() -> Result<()> {
    let src = paths::census_crate().join("src");
    let rules = Rules::compile()?;
    let (modules, references) = walk::tree(&src, &rules)?;

    let mut grouped: BTreeMap<(String, String), Vec<&Reference>> = BTreeMap::new();
    for reference in &references {
        grouped
            .entry((reference.from.clone(), reference.to.clone()))
            .or_default()
            .push(reference);
    }

    let mut edges = Vec::new();
    let mut violations = Vec::new();
    for ((from, to), group) in &grouped {
        edges.push(json!({"from": from, "to": to, "refs": count(group.len())}));
        if !is_allowed(from, to) {
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
    let violation_count = violations.len();
    let report = json!({
        "modules": modules,
        "edges": edges,
        "violations": violations,
    });
    println!("{}", serde_json::to_string_pretty(&report)?);
    if violation_count > 0 {
        bail!("{violation_count} module-seam violation(s): an edge outside the allowed table");
    }
    Ok(())
}

/// Whether `(from, to)` is a declared edge of the census module graph.
fn is_allowed(from: &str, to: &str) -> bool {
    ALLOWED
        .iter()
        .any(|(allowed_from, allowed_to)| *allowed_from == from && *allowed_to == to)
}
