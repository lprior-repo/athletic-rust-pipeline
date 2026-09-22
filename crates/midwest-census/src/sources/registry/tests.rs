//! The registry's own contract: unique slugs, an admission inside the collection ceiling, a lookup
//! that answers every registered slug, and a planning order that puts bulk payloads first.
//!
//! These run against the table itself rather than a fixture, so a table edit that breaks one of the
//! properties fails here. Each test names the property it holds, because the point is to catch a
//! broken table, not to restate its contents.

use super::{bulk_first, descriptor, TransportKind, REGISTRY};

/// The vocabulary a plan can choose from: one slug per adapter that fetches or parses external
/// source material, which is also the name the provider dispatch accepts (§11). Spelled out
/// deliberately, so deleting an entry fails here instead of quietly shrinking what a plan can see,
/// and adding one requires saying so in both places. An adapter's stamped evidence id is not a plan
/// name (`wayzata_schedule` for `wayzata`, `ohsaa_portal` for `ohsaa`), so only slugs appear here.
const PLAN_SLUGS: [&str; 14] = [
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
    "tfrrs",
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
        PLAN_SLUGS.len(),
        "the table and its list of provider modules disagree"
    );
    for declared in PLAN_SLUGS {
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

/// The transport grants one in-flight request per host, and every entry repeats that bound so a
/// widened one has to be a visible edit in the table rather than a silent one in a caller. The same
/// holds for the crawl-delay floor: the fetcher raises spacing to a published `Crawl-delay` and
/// never lowers it, so an entry claiming otherwise would describe a run the transport refuses.
#[test]
fn every_admission_repeats_the_transport_bound() {
    for entry in REGISTRY {
        assert_eq!(
            entry.admission.maximum_in_flight.get(),
            1,
            "{} declares more than one in-flight request for {}",
            entry.slug,
            entry.admission.origin
        );
        assert!(
            entry.admission.robots_crawl_delay_respected,
            "{} declares that a published crawl-delay is ignored",
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
/// source that only names athletes, whatever order the caller listed the two in.
///
/// `athleticnet` answers both shapes now (bio per athlete, whole meet per pull), so it can no
/// longer serve as the weaker side: it sorts by its bulk route, which the last assertion holds to.
#[test]
fn bulk_first_puts_a_bulk_source_before_an_athlete_index() {
    let plan = bulk_first(&["athleticlive_athletes", "wiaa_results"]);
    assert_eq!(plan.first().map(|entry| entry.slug), Some("wiaa_results"));
    assert_eq!(
        plan.last().map(|entry| entry.slug),
        Some("athleticlive_athletes")
    );
    assert_eq!(bulk_first(&["wiaa_results", "athleticlive_athletes"]), plan);
    assert_eq!(
        bulk_first(&["athleticnet", "wiaa_results"])
            .first()
            .map(|entry| entry.slug),
        Some("athleticnet"),
        "a source with a bulk route sorts by that route, not by its weaker one"
    );
}

/// The bands run bulk results, then meet discovery, then athlete-shaped sources, then everything
/// else — one source from each band, so a band that swallowed its neighbour shows up here.
///
/// `milesplit` is deliberately not the athlete-shaped probe: the `/raw` route makes it a
/// `bulk_results` source, which the runner below would otherwise be testing for the wrong band.
#[test]
fn bands_run_bulk_meet_athlete_then_directories() {
    let plan = bulk_first(&["mshsl", "athleticlive_athletes", "wayzata", "wiaa_results"]);
    let slugs: Vec<&str> = plan.iter().map(|entry| entry.slug).collect();
    assert_eq!(
        slugs,
        ["wiaa_results", "wayzata", "athleticlive_athletes", "mshsl"],
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

/// Every name a plan can choose has to survive both queries the planner calls: `descriptor` answers
/// it with its own entry, and `bulk_first` keeps it in the plan instead of filtering it out. A slug
/// that is registered but dropped from the ordering would be a source an operator can select and
/// the plan then silently ignores.
#[test]
fn every_plan_slug_resolves_and_stays_in_the_plan() {
    for slug in PLAN_SLUGS {
        assert_eq!(
            descriptor(slug).map(|entry| entry.slug),
            Some(slug),
            "{slug} is a plan name with no descriptor"
        );
        let plan = bulk_first(&[slug]);
        assert_eq!(
            plan.iter().map(|entry| entry.slug).collect::<Vec<&str>>(),
            vec![slug],
            "{slug} dropped out of its own plan"
        );
    }
}

/// Multiple entries behind one origin each repeat that origin's policy, so a rate that drifted on
/// one of them shows up here as a disagreement instead of as traffic the host never permitted, and
/// a widened in-flight bound has to be an edit every entry agrees on.
#[test]
fn one_origin_has_one_declared_policy() {
    let mut seen: Vec<(&str, f64, usize)> = Vec::new();
    for entry in REGISTRY {
        let admission = entry.admission;
        let in_flight = admission.maximum_in_flight.get();
        let Some((_, rate, bound)) = seen.iter().find(|(origin, ..)| *origin == admission.origin)
        else {
            seen.push((
                admission.origin,
                admission.target_requests_per_second,
                in_flight,
            ));
            continue;
        };
        assert_eq!(
            *rate, admission.target_requests_per_second,
            "{} declares a different rate than another entry for {}",
            entry.slug, admission.origin
        );
        assert_eq!(
            *bound, in_flight,
            "{} declares a different in-flight bound than another entry for {}",
            entry.slug, admission.origin
        );
    }
}
