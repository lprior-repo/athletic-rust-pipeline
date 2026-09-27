
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

fn is_allowed(from: &str, to: &str) -> bool {
    ALLOWED
        .iter()
        .any(|(allowed_from, allowed_to)| *allowed_from == from && *allowed_to == to)
}

fn is_crate_allowed(from: &str, to: &str) -> bool {
    ALLOWED_CRATES
        .iter()
        .any(|(allowed_from, allowed_to)| *allowed_from == from && *allowed_to == to)
}
