use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use super::Check;

const DOMAIN: &str = "census-domain";

const LAYERS: [(&str, u32); 9] = [
    ("census-domain", 0),
    ("census-store", 1),
    ("census-crawl", 2),
    ("athleticnet-browser", 2),
    ("census-reconcile", 3),
    ("census-review", 3),
    ("census-report", 4),
    ("census-service", 5),
    ("xtask", 6),
];

const DOMAIN_FORBIDDEN: [&str; 5] = ["tokio", "reqwest", "fjall", "restate-sdk", "chromiumoxide"];

const TOOLING_FORBIDDEN: [&str; 4] = [
    "census-crawl",
    "census-report",
    "census-service",
    "census-store",
];

const KNOWN_UPWARD: [(&str, &str); 5] = [
    ("census-reconcile", "census-report"),
    ("xtask", "census-crawl"),
    ("xtask", "census-report"),
    ("xtask", "census-service"),
    ("xtask", "census-store"),
];

fn known_upward(from: &str, to: &str) -> bool {
    KNOWN_UPWARD
        .iter()
        .any(|(known_from, known_to)| *known_from == from && *known_to == to)
}

fn note_edge(from: &str, to: &str, failures: &mut Vec<String>, deviations: &mut Vec<String>) {
    if known_upward(from, to) {
        deviations.push(format!("{from} -> {to} (known)"));
        return;
    }
    failures.push(format!(
        "{from} -> {to} crosses a crate boundary the architecture does not grant; add it to KNOWN_UPWARD only with a recorded reason"
    ));
}

pub(super) fn crate_boundaries() -> Result<Check> {
    const NAME: &str = "crate boundaries";
    let members = members()?;
    let mut failures: Vec<String> = Vec::new();
    let mut deviations: Vec<String> = Vec::new();

    for (name, dependencies) in &members {
        let internal: Vec<&String> = dependencies
            .iter()
            .filter(|dependency| members.contains_key(*dependency))
            .collect();

        if name == DOMAIN {
            for dependency in dependencies {
                if DOMAIN_FORBIDDEN.contains(&dependency.as_str())
                    || members.contains_key(dependency)
                {
                    failures.push(format!(
                        "{DOMAIN} depends on {dependency}; the domain crate owns pure constrained types only"
                    ));
                }
            }
        }

        if name == "xtask" {
            for dependency in &internal {
                if TOOLING_FORBIDDEN.contains(&dependency.as_str()) {
                    note_edge(name, dependency, &mut failures, &mut deviations);
                }
            }
        }

        let own = layer(name);
        for dependency in internal {
            match (own, layer(dependency)) {
                (Some(own), Some(dependency_layer)) if dependency_layer > own => {
                    note_edge(name, dependency, &mut failures, &mut deviations);
                }
                (Some(_), Some(_)) => {}
                _ => failures.push(format!(
                    "{name} or {dependency} is not placed in the crate layering"
                )),
            }
        }
    }

    failures.extend(cycles(&members));

    let edges: usize = members
        .values()
        .map(|dependencies| {
            dependencies
                .iter()
                .filter(|dependency| members.contains_key(*dependency))
                .count()
        })
        .sum();
    let detail = format!(
        "{} packages, {edges} internal edges, {} known deviation(s)",
        members.len(),
        deviations.len()
    );

    if !failures.is_empty() {
        return Ok(Check::violated(10, NAME, detail, failures));
    }
    if !deviations.is_empty() {
        let joined = deviations.join("; ");
        return Ok(Check::deviates(10, NAME, detail, joined));
    }
    Ok(Check::holds(10, NAME, detail))
}

fn layer(name: &str) -> Option<u32> {
    LAYERS
        .iter()
        .find(|(candidate, _)| *candidate == name)
        .map(|(_, layer)| *layer)
}

fn members() -> Result<BTreeMap<String, BTreeSet<String>>> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .context("running cargo metadata for the crate boundary check")?;
    if !output.status.success() {
        bail!("cargo metadata exited {}", output.status);
    }
    let value: Value =
        serde_json::from_slice(&output.stdout).context("parsing cargo metadata output")?;
    let packages = value
        .get("packages")
        .and_then(Value::as_array)
        .context("cargo metadata reported no packages")?;
    let mut workspace = BTreeMap::new();
    for package in packages {
        let name = package
            .get("name")
            .and_then(Value::as_str)
            .context("a package has no name")?
            .to_string();
        let mut dependencies = BTreeSet::new();
        for dependency in package
            .get("dependencies")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(dependency) = dependency.get("name").and_then(Value::as_str) {
                dependencies.insert(dependency.to_string());
            }
        }
        workspace.insert(name, dependencies);
    }
    Ok(workspace)
}

fn cycles(members: &BTreeMap<String, BTreeSet<String>>) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for name in members.keys() {
        let mut seen: BTreeSet<String> = BTreeSet::new();
        if reaches_self(name, name, members, &mut seen) {
            found.push(format!(
                "{name} reaches itself through internal dependencies"
            ));
        }
    }
    found.sort();
    found.dedup();
    found
}

fn reaches_self(
    start: &str,
    current: &str,
    members: &BTreeMap<String, BTreeSet<String>>,
    seen: &mut BTreeSet<String>,
) -> bool {
    let Some(dependencies) = members.get(current) else {
        return false;
    };
    for dependency in dependencies {
        if !members.contains_key(dependency) || !seen.insert(dependency.clone()) {
            continue;
        }
        if dependency == start || reaches_self(start, dependency, members, seen) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::{cycles, layer, note_edge, reaches_self};
    use std::collections::{BTreeMap, BTreeSet};

    fn graph(edges: &[(&str, &str)]) -> BTreeMap<String, BTreeSet<String>> {
        let mut members: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (from, to) in edges {
            members
                .entry((*from).to_string())
                .or_default()
                .insert((*to).to_string());
            members.entry((*to).to_string()).or_default();
        }
        members
    }

    #[test]
    fn the_declared_layering_places_every_workspace_crate() {
        assert!(layer("census-domain") < layer("census-store"));
        assert!(layer("census-store") < layer("census-crawl"));
        assert!(layer("census-report") < layer("census-service"));
        assert!(layer("unknown-crate").is_none());
    }

    #[test]
    fn a_cycle_is_reported_once_per_participant() {
        let members = graph(&[("a", "b"), ("b", "c"), ("c", "a"), ("d", "b")]);
        let found = cycles(&members);
        assert_eq!(found.len(), 3);
        assert!(found.iter().all(|entry| entry.contains("reaches itself")));
    }

    #[test]
    fn a_diamond_is_not_a_cycle() {
        let members = graph(&[("a", "b"), ("a", "c"), ("b", "d"), ("c", "d")]);
        assert!(cycles(&members).is_empty());
        let mut seen = BTreeSet::new();
        assert!(!reaches_self("a", "a", &members, &mut seen));
    }

    #[test]
    fn a_recorded_boundary_crossing_is_reported_but_a_new_one_fails() {
        let mut failures = Vec::new();
        let mut deviations = Vec::new();
        note_edge(
            "census-reconcile",
            "census-report",
            &mut failures,
            &mut deviations,
        );
        assert!(failures.is_empty());
        assert_eq!(deviations.len(), 1);
        note_edge(
            "census-crawl",
            "census-service",
            &mut failures,
            &mut deviations,
        );
        assert_eq!(failures.len(), 1);
        assert_eq!(deviations.len(), 1);
    }
}
