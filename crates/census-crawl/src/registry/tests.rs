use super::{
    bulk_first, declared_delay_for_host, descriptor, descriptors, transport_for_host, AccessClass,
    TransportKind,
};
use std::time::Duration;

const PLAN_SLUGS: [&str; 19] = [
    "athleticlive",
    "athleticlive_athletes",
    "athleticnet",
    "ciac",
    "chsaa",
    "coach_contacts",
    "coach_directories",
    "ihsa",
    "ks",
    "milesplit",
    "mpa",
    "mshsl",
    "ohsaa",
    "plain_names",
    "riil",
    "tfrrs",
    "wiaa",
    "wiaa_results",
    "wayzata",
];

#[test]
fn every_provider_module_is_registered_exactly_once() {
    let mut slugs: Vec<&str> = descriptors().map(|entry| entry.slug).collect();
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

#[test]
fn descriptor_answers_every_registered_slug_and_nothing_else() {
    for entry in descriptors() {
        let found = descriptor(entry.slug);
        assert!(found.is_some(), "{} has no lookup answer", entry.slug);
        assert_eq!(found.map(|found| found.slug), Some(entry.slug));
        assert_eq!(found.map(|found| found.provider), Some(entry.provider));
    }
    assert!(descriptor("no_such_source").is_none());
    assert!(descriptor("").is_none());
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

#[test]
fn every_admission_stays_inside_the_collection_ceiling() {
    for entry in descriptors() {
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

#[test]
fn every_admission_repeats_the_transport_bound() {
    for entry in descriptors() {
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

#[test]
fn access_class_separates_artifact_reads_from_host_fetches() {
    for slug in ["athleticlive", "coach_contacts"] {
        assert_eq!(
            descriptor(slug).map(|entry| entry.access_class()),
            Some(AccessClass::Artifact),
            "{slug} reads an artifact and costs no request"
        );
    }
    let fetched: Vec<&str> = descriptors()
        .filter(|entry| entry.access_class() != AccessClass::Artifact)
        .map(|entry| entry.slug)
        .collect();
    assert_eq!(
        fetched.len(),
        descriptors().count() - 2,
        "only the two artifact adapters may skip the fetch route: {fetched:?}"
    );
    let mut classes: Vec<AccessClass> = Vec::new();
    for slug in fetched {
        let entry = descriptor(slug).expect("a fetched slug names a descriptor");
        let expected = match entry.transport {
            TransportKind::Browser => AccessClass::BrowserSession,
            _ => AccessClass::Open,
        };
        assert_eq!(
            entry.access_class(),
            expected,
            "{slug} is classed by its own transport"
        );
        classes.push(expected);
    }
    assert!(
        classes.contains(&AccessClass::BrowserSession),
        "a plan field no source can reach is a class nobody records: {classes:?}"
    );
}

#[test]
fn access_class_names_are_stable() {
    assert_eq!(AccessClass::Open.as_str(), "open");
    assert_eq!(AccessClass::Artifact.as_str(), "artifact");
    assert_eq!(AccessClass::BrowserSession.as_str(), "browser_session");
}

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

#[test]
fn one_origin_has_one_declared_policy() {
    let mut seen: Vec<(&str, f64, usize)> = Vec::new();
    for entry in descriptors() {
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

#[test]
fn one_origin_has_one_transport() {
    let mut seen: Vec<(&str, TransportKind)> = Vec::new();
    for entry in descriptors() {
        let origin = entry.admission.origin;
        match seen.iter().find(|(seen_origin, _)| *seen_origin == origin) {
            Some((_, seen_transport)) => assert_eq!(
                *seen_transport, entry.transport,
                "{} declares a different transport than another entry for {origin}",
                entry.slug
            ),
            None => seen.push((origin, entry.transport)),
        }
    }
}

#[test]
fn the_declared_rate_for_a_host_is_read_from_the_table() {
    assert_eq!(
        declared_delay_for_host("www.mpa.cc"),
        Some(Duration::from_secs(1)),
        "a one-request-per-second origin paces at one second"
    );
    assert_eq!(
        declared_delay_for_host("www.wayzataresults.com"),
        Some(Duration::from_secs(10)),
        "the crawl-delay origin paces at the rate its row declares"
    );
    assert_eq!(
        declared_delay_for_host("WWW.WayzataResults.COM"),
        Some(Duration::from_secs(10))
    );
    assert_eq!(
        declared_delay_for_host("example.invalid"),
        None,
        "an unregistered host declares no rate for the fetcher to floor against"
    );
    assert_eq!(
        declared_delay_for_host("local-artifact"),
        None,
        "an artifact origin is not a host to pace"
    );
}

#[test]
fn the_transport_lookup_reads_the_table() {
    assert_eq!(
        transport_for_host("www.athletic.net"),
        Some(TransportKind::Browser),
        "Athletic.net is the source policy's browser-lane source"
    );
    assert_eq!(
        transport_for_host("WWW.Athletic.NET"),
        Some(TransportKind::Browser),
        "a URL's host and the row's origin need not be spelled the same way"
    );
    assert_eq!(
        transport_for_host("example.invalid"),
        None,
        "a host no descriptor claims is not this table's to route"
    );
}
