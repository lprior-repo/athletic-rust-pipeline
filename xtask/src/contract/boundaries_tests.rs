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

#[test]
fn domain_dependency_failures_retain_original_check_order_and_exact_messages() {
    let members = graph(&[("census-domain", "census-store")]);
    let dependencies = BTreeSet::from([
        "census-store".to_string(),
        "reqwest".to_string(),
        "tokio".to_string(),
    ]);
    let mut failures = Vec::new();
    let mut deviations = Vec::new();
    super::check_package_edges(
        "census-domain",
        &dependencies,
        &members,
        &mut failures,
        &mut deviations,
    );
    assert_eq!(failures, [
        "census-domain depends on census-store; the domain crate owns pure constrained types only",
        "census-domain depends on reqwest; the domain crate owns pure constrained types only",
        "census-domain depends on tokio; the domain crate owns pure constrained types only",
        "census-domain -> census-store crosses a crate boundary the architecture does not grant; add it to KNOWN_UPWARD only with a recorded reason",
    ]);
    assert_eq!(deviations, Vec::<String>::new());
}

#[test]
fn tooling_dependencies_keep_each_known_deviation_once_without_upward_double_counting() {
    let members = graph(&[
        ("xtask", "census-crawl"),
        ("xtask", "census-report"),
        ("xtask", "census-service"),
        ("xtask", "census-store"),
    ]);
    let mut failures = Vec::new();
    let mut deviations = Vec::new();
    let dependencies = BTreeSet::from([
        "census-crawl".to_string(),
        "census-report".to_string(),
        "census-service".to_string(),
        "census-store".to_string(),
    ]);
    super::check_package_edges(
        "xtask",
        &dependencies,
        &members,
        &mut failures,
        &mut deviations,
    );
    assert_eq!(failures, Vec::<String>::new());
    assert_eq!(
        deviations,
        [
            "xtask -> census-crawl (known)",
            "xtask -> census-report (known)",
            "xtask -> census-service (known)",
            "xtask -> census-store (known)",
        ]
    );
}
