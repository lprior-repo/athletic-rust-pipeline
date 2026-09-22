//! The registry's own contract: unique slugs, an admission inside the collection ceiling, a lookup
//! that answers every registered slug, and a planning order that puts bulk payloads first.
//!
//! These run against the table itself rather than a fixture, so a table edit that breaks one of the
//! properties fails here. Each test names the property it holds, because the point is to catch a
//! broken table, not to restate its contents.

use super::{bulk_first, descriptor, TransportKind, REGISTRY};

/// The provider modules this registry exists for: one per adapter that fetches or parses external
/// source material. Spelled out deliberately, so deleting an entry fails here instead of quietly
/// shrinking what a plan can see, and adding one requires saying so in both places.
const PROVIDER_SLUGS: [&str; 13] = [
    "athleticlive",
    "athleticlive_athletes",
    "athleticnet",
    "coach_contacts",
    "ihsa",
    "ks",
    "milesplit",
    "mshsl",
    "ohsaa",
    "plain_names",
    "wiaa",
    "wiaa_results",
    "wayzata",
];

/// Two entries under one slug would make `descriptor(slug)` answer with whichever the scan met
/// first, and a declared-but-missing module would be a provider no plan can select.
#[test]
fn every_provider_module_is_registered_exactly_once() {
    let mut slugs: Vec<&str> = REGISTRY.iter().map(|entry| entry.slug).collect();
    let listed = slugs.len();
    slugs.sort_unstable();
    let mut unique = slugs.clone();
    unique.dedup();
    assert_eq!(
        unique.len(),
        listed,
        "the table registers one slug more than once"
    );
    assert_eq!(
        unique.len(),
        PROVIDER_SLUGS.len(),
        "the table and its list of provider modules disagree"
    );
    for declared in PROVIDER_SLUGS {
        assert!(
            unique.binary_search(&declared).is_ok(),
            "{declared} is not registered"
        );
    }
}

/// The lookup is the table's only index: every registered slug has to answer with its own entry,
/// and a slug naming no source has to answer with nothing rather than with a neighbour.
#[test]
fn descriptor_answers_every_registered_slug_and_nothing_else() {
    for entry in REGISTRY {
        let found = descriptor(entry.slug);
        assert!(found.is_some(), "{} has no lookup answer", entry.slug);
        assert_eq!(found.map(|found| found.slug), Some(entry.slug));
        assert_eq!(found.map(|found| found.provider), Some(entry.provider));
    }
    assert!(descriptor("no_such_source").is_none());
    assert!(descriptor("").is_none());
    // `wiaa` is a prefix of `wiaa_results`, and `athleticlive` of `athleticlive_athletes`: a lookup
    // that matched on a prefix would answer both with the wrong source.
    assert_eq!(descriptor("wiaa").map(|entry| entry.slug), Some("wiaa"));
    assert_eq!(
        descriptor("wiaa_results").map(|entry| entry.slug),
        Some("wiaa_results")
    );
    assert_eq!(
        descriptor("athleticlive").map(|entry| entry.slug),
        Some("athleticlive")
    );
    assert_eq!(
        descriptor("athleticlive_athletes").map(|entry| entry.slug),
        Some("athleticlive_athletes")
    );
    assert!(descriptor("wiaa_").is_none());
}

/// The collection's ceiling is 2 rps per host (`net`'s authorized-host floor), so a declaration
/// above it would be a rate the transport never grants; a declaration of zero or less would be a
/// source no request may ever be made against.
#[test]
fn every_admission_stays_inside_the_collection_ceiling() {
    for entry in REGISTRY {
        let declared = entry.admission.target_requests_per_second;
        assert!(
            declared > 0.0,
            "{} declares {declared} rps, which is not a request rate",
            entry.slug
        );
        assert!(
            declared <= 2.0,
            "{} declares {declared} rps, above the 2 rps collection ceiling",
            entry.slug
        );
        assert!(
            !entry.admission.origin.is_empty(),
            "{} declares no origin for its admission",
            entry.slug
        );
    }
}

/// The configured spacing is one second by default, and a host whose `robots.txt` asks for ten is
/// held to ten: a declaration that kept the default there would contradict the floor the fetcher
/// applies, which is exactly the mismatch this field exists to make reviewable.
#[test]
fn a_crawl_delay_host_declares_the_slower_rate() {
    assert_eq!(
        descriptor("wayzata").map(|entry| entry.admission.target_requests_per_second),
        Some(0.1)
    );
    assert_eq!(
        descriptor("mshsl").map(|entry| entry.admission.target_requests_per_second),
        Some(1.0)
    );
}

/// An adapter that reads a checked-in artifact contacts no host, so declaring a live origin for it
/// would let a plan believe a request is possible where none is.
#[test]
fn artifact_adapters_declare_no_fetchable_host() {
    for slug in ["athleticlive", "coach_contacts"] {
        let entry = descriptor(slug);
        assert_eq!(
            entry.map(|entry| entry.transport),
            Some(TransportKind::Csv),
            "{slug} reads an artifact"
        );
        assert_eq!(
            entry.map(|entry| entry.admission.origin),
            Some("local-artifact"),
            "{slug} names no host to pace"
        );
    }
}

/// ADR-004's preference in one assertion: a payload that carries many performances comes before a
/// per-athlete profile, whatever order the caller listed the two in.
#[test]
fn bulk_first_puts_a_bulk_source_before_a_profile_only_one() {
    let plan = bulk_first(&["athleticnet", "wiaa_results"]);
    assert_eq!(plan.first().map(|entry| entry.slug), Some("wiaa_results"));
    assert_eq!(plan.last().map(|entry| entry.slug), Some("athleticnet"));
    assert_eq!(bulk_first(&["wiaa_results", "athleticnet"]), plan);
}

/// The bands run bulk results, then meet discovery, then athlete-shaped sources, then everything
/// else — one source from each band, so a band that swallowed its neighbour shows up here.
#[test]
fn bands_run_bulk_meet_athlete_then_directories() {
    let plan = bulk_first(&["mshsl", "milesplit", "wayzata", "wiaa_results"]);
    let slugs: Vec<&str> = plan.iter().map(|entry| entry.slug).collect();
    assert_eq!(
        slugs,
        ["wiaa_results", "wayzata", "milesplit", "mshsl"],
        "one source per band, in band order"
    );
}

/// Ordering inside a band is by slug rather than by caller order, so a plan is reproducible from the
/// same set of slugs; a repeated slug contributes one source, not two places in a budget; and a slug
/// nobody registered contributes nothing.
#[test]
fn equal_bands_order_by_slug_and_ignore_repeats() {
    let forward = bulk_first(&["ihsa", "ks", "ohsaa", "wiaa", "plain_names"]);
    let backward = bulk_first(&["wiaa", "plain_names", "ohsaa", "ks", "ihsa"]);
    assert_eq!(forward, backward, "the plan depends on the caller's order");
    let slugs: Vec<&str> = forward.iter().map(|entry| entry.slug).collect();
    assert_eq!(slugs, ["ihsa", "ks", "ohsaa", "plain_names", "wiaa"]);
    assert_eq!(bulk_first(&["ihsa", "ihsa"]).len(), 1);
    assert!(bulk_first(&["no_such_source"]).is_empty());
}
