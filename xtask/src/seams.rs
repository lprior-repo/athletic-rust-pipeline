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
    // The serving region: `bootstrap` binds the durable ingress endpoint, so it names the router it
    // serves and drains here. That is composition-root wiring rather than a lane edge — `ingress`
    // names nothing back — and the endpoint's own task and shutdown live in this module.
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
    // The acquisition plane: the polite fetcher, the browser bridge, one module per provider and the
    // provider registry. It is below the run — it maps provider data onto domain rows and writes them
    // through the store — and names nothing above it, so the run's shape and the durable layer's shape
    // cannot leak back into an adapter. `net`'s one recorded exception, reading the registry to learn
    // which transport carries a host, is now a lookup inside this crate rather than an edge between
    // two: the browser bridge's `lane.rs` reaches the Restate ingress client as a library, not as the
    // workspace's service definitions, which is why the SDK appears here as a dependency and the
    // service crate does not.
    ("census-crawl", "census-domain"),
    // The fetch cache, the request-evidence tables and the clock the net layer stamps a crawl with.
    ("census-crawl", "census-store"),
    // The bottom of the graph: types and their rules, no store, no network, no runtime.
    ("census-review", "census-domain"),
    // The review lane reads retained cases and writes verdicts through the store that owns both
    // tables; it never opens Fjall itself.
    ("census-review", "census-store"),
    // The reconciliation lane: the derive pass, deterministic workflow identity, which is a pure
    // function of explicit values, and the row-level check that holds the workbook against the store.
    // It reads the domain and the store and names nothing back, so a projection's verdict cannot
    // depend on the service that published it. Its one outward edge is the reporting plane's own
    // rendering — the coverage report and the families the workbook sheets print — reused *because*
    // the derived tables and the workbook must not be shown two different findings; it is one-way,
    // because the reporting plane names no reconciliation module.
    ("census-reconcile", "census-domain"),
    ("census-reconcile", "census-report"),
    ("census-reconcile", "census-store"),
    // The reporting plane: coverage, per-athlete bests and the workbook export. It reads the canonical
    // model and the store's read model, renders the review lane's retained families as its conflict and
    // review sheets, and names the acquisition plane only for the adapter surface the meta sheets print
    // as data. It acquires nothing, so nothing above it can leak into a projection.
    ("census-report", "census-crawl"),
    ("census-report", "census-domain"),
    ("census-report", "census-review"),
    ("census-report", "census-store"),
    ("census-store", "census-domain"),
    // The composition root: it owns the sweep, the durable services and the published artifacts, and it
    // is the only crate that may name the acquisition plane, the review lane and the store together.
    //
    // The three edges carry the contracts that used to be module rows:
    //
    // * `census-crawl` — ARCHITECTURE.md §1: the batch path and the durable path share the adapters and
    //   differ only in who owns the journal, so the workflow layer names the adapter error type it
    //   classifies into the durable retry policy (`jobs::collect_error`) and the polite fetcher the
    //   jurisdiction object holds for the whole process (§3, one origin drawing from one admission
    //   budget). The workbook's meta sheets render the adapter surface itself — slug, transport,
    //   declared capabilities, per-origin cost — so they read the registry table as data and never
    //   call an adapter. The provenance gate (`coachverify`) fetches cited pages through the same
    //   fetcher, three passes over one shared cache.
    // * `census-review` — the athlete key (`school, normalized name, cohort`) has one definition, in
    //   the review lane's flags, and the conflict queue groups by that same function rather than
    //   stating a second copy that could drift from the flags the lane states to a model. Nothing else
    //   of the lane is named: the queue reads the key and the store's rows.
    // * `census-store` — every derived artifact is published the way the store publishes one: through
    //   its atomic-rename writer and its CSV failure type, so a reader never sees a half-written state
    //   file and one publication bug has one implementation. `bests`, `index`, `report` and the coach
    //   fragments all take that edge for the writer plumbing and read no row through it.
    // The composition root also hosts the browser lane. The persistent headed session is a durable
    // service — a Restate object owns the profile, so the process that serves it must be the one
    // holding the lane (`restate_services/browser_session.rs`, its `mod.rs` clock, the CLI client and
    // the bootstrap options). `census-crawl` owns the *protocol* side of that lane and reaches the
    // ingress as a library, this crate owns the *host*, and the edge closes no loop: the lane crate
    // declares no workspace dependency at all, so the one-way property below still holds.
    ("census-service", "athleticnet-browser"),
    ("census-service", "census-crawl"),
    // The reconciliation lane is named by the durable services, which derive each workflow's identity
    // from it, and by the `verify` verb that runs its row-level check. It names nothing back.
    ("census-service", "census-reconcile"),
    // The reporting plane is the composition root's projection layer: the CLI's export and seal verbs
    // and the durable workbook service name it, and it names nothing back.
    ("census-service", "census-report"),
    ("census-service", "census-domain"),
    ("census-service", "census-review"),
    ("census-service", "census-store"),
    // The harness reads the store for its status verb, drives the census services through their
    // ingress clients, replays a provider adapter against its capture, and crosses the report scope
    // when it resolves a run's artifact paths, so it names all four.
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
