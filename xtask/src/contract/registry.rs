mod origin;

use anyhow::{Context, Result};
use census_crawl::registry::{descriptor, descriptors};
use census_crawl::TransportKind;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;

use super::Check;
use crate::paths;

use self::origin::is_host;

const ATHLETICNET: &str = "athleticnet";

pub(super) const DEVIATIONS: [(&str, TransportKind, &str); 1] = [(
    ATHLETICNET,
    TransportKind::StructuredApi,
    "the Athletic.net browser migration has not landed: the adapter still arrives through the JSON client, and this check reports the deviation rather than failing the gate until it does",
)];

pub(super) fn athleticnet_transport() -> Result<Check> {
    const NAME: &str = "athleticnet transport";
    let Some(source) = descriptor(ATHLETICNET) else {
        return Ok(Check::violated(
            2,
            NAME,
            format!("no descriptor has the slug `{ATHLETICNET}`"),
            vec![format!(
                "the registry must carry the Athletic.net adapter under the slug `{ATHLETICNET}`"
            )],
        ));
    };
    let detail = format!(
        "`{ATHLETICNET}` ({}) arrives as {:?}",
        source.provider, source.transport
    );
    if source.transport == TransportKind::Browser {
        return Ok(Check::holds(2, NAME, detail));
    }
    let recorded = DEVIATIONS
        .iter()
        .find(|(slug, registered, _)| *slug == ATHLETICNET && *registered == source.transport);
    if let Some((_, _, reason)) = recorded {
        return Ok(Check::deviates(
            2,
            NAME,
            detail,
            format!("`{ATHLETICNET}` must arrive as TransportKind::Browser; {reason}"),
        ));
    }
    Ok(Check::violated(
        2,
        NAME,
        detail,
        vec![format!(
            "the Athletic.net adapter arrives as {:?}, which is neither the browser session this check asserts nor a transport the deviation table records",
            source.transport
        )],
    ))
}

pub(super) fn admissions() -> Result<Check> {
    const NAME: &str = "source admission";
    let mut failures: Vec<String> = Vec::new();
    let mut count = 0usize;
    let mut origins: BTreeSet<&'static str> = BTreeSet::new();
    let mut rates: BTreeSet<String> = BTreeSet::new();
    let mut in_flight: BTreeSet<usize> = BTreeSet::new();
    for source in descriptors() {
        count = count.saturating_add(1);
        let admission = source.admission;
        origins.insert(admission.origin);
        rates.insert(format!("{}", admission.target_requests_per_second));
        in_flight.insert(admission.maximum_in_flight.get());
        if !is_host(admission.origin) {
            failures.push(format!(
                "{}: the admission origin {:?} is not a host",
                source.slug, admission.origin
            ));
        }
        let rate = admission.target_requests_per_second;
        if rate.is_nan() || rate <= 0.0 {
            failures.push(format!(
                "{}: target_requests_per_second is {}, which is not a positive rate ({})",
                source.slug, admission.target_requests_per_second, admission.origin
            ));
        }
        if admission.maximum_in_flight.get() < 1 {
            failures.push(format!(
                "{}: maximum_in_flight is below the one request the fetcher grants a host",
                source.slug
            ));
        }
    }
    let detail = format!(
        "{count} descriptors over {} origins [{}], {} rps, {} in flight",
        origins.len(),
        origins.iter().copied().collect::<Vec<&str>>().join(", "),
        rates.iter().cloned().collect::<Vec<String>>().join("/"),
        in_flight
            .iter()
            .map(usize::to_string)
            .collect::<Vec<String>>()
            .join("/"),
    );
    if count == 0 {
        failures.push("the registry lists no source descriptor at all".to_string());
    }
    if failures.is_empty() {
        return Ok(Check::holds(5, NAME, detail));
    }
    Ok(Check::violated(5, NAME, detail, failures))
}

const NON_ADAPTERS: [(&str, &str); 17] = [
    (
        "applicability",
        "the per-jurisdiction source table the planner reads: data, with no origin to admit",
    ),
    ("compiled", "parses the `Compiled` timer export family"),
    (
        "directory",
        "the shared school-directory reader contract (`ReadOutcome`, its skip ledger and the field mappers every directory reader builds rows with): it admits no origin of its own",
    ),
    (
        "geocode",
        "the Google geocoder and USPS address validator the corpus verb reaches when the operator supplies credentials: typed clients of external APIs, with no source origin to admit",
    ),
    ("hytek", "parses Hy-Tek Meet Manager result files"),
    (
        "private_assoc",
        "reads association membership listings the operator supplies: no capture and no robots verdict exist for the association hosts, so it has no descriptor and is reachable only through the corpus verb",
    ),
    ("raceday", "parses RaceDay Scoring result exports"),
    (
        "row_hygiene",
        "shared person, school, vendor and varsity admission rules; no source origin",
    ),
    (
        "result_file",
        "the result-file domain model every vendor parser shares",
    ),
    (
        "athlete_observations",
        "the athlete half of the observation funnel: it appends `SourceObservations` rows for the athletes a pass read, through whatever adapter read them, and admits no origin of its own",
    ),
    (
        "xc",
        "parses the cross-country result files WIAA's timers publish",
    ),
    (
        "context",
        "the per-pass context a crawl walk borrows: the row sink and the reader in one value, admitting no origin of its own",
    ),
    (
        "recording",
        "the row sink the walks write through: it records what a pass acquired so the caller can post it as the object's own acquisition, and admits no origin",
    ),
    (
        "registry",
        "the capability registry the adapters are declared in",
    ),
    (
        "net",
        "the polite fetcher and the browser bridge every adapter draws on",
    ),
    (
        "ingress",
        "the loopback client that submits pipeline work to the serving census through Restate instead of opening the store: it admits no origin of its own",
    ),
    (
        "lib",
        "the crate root: the module list itself, not an adapter",
    ),
];

pub(super) fn adapter_registration() -> Result<Check> {
    const NAME: &str = "adapter registration";
    let registered: BTreeSet<&str> = descriptors().map(|source| source.slug).collect();
    let modules = source_modules()?;
    let mut failures: Vec<String> = Vec::new();
    let mut matched = 0usize;
    let mut readers = 0usize;
    for module in &modules {
        if registered.contains(module.as_str()) {
            matched = matched.saturating_add(1);
            continue;
        }
        if NON_ADAPTERS.iter().any(|(name, _)| name == module) {
            readers = readers.saturating_add(1);
            continue;
        }
        failures.push(format!(
            "`{module}` is a module in the crawl crate with no registered descriptor: either an adapter the planner cannot see, or a reader that belongs in the non-adapter list"
        ));
    }
    for slug in &registered {
        if !modules.contains(*slug) {
            failures.push(format!(
                "the descriptor `{slug}` has no `{slug}` module in the crawl crate to dispatch to"
            ));
        }
    }
    if modules.is_empty() {
        failures.push("the crawl crate's source root listed no module at all".to_string());
    }
    let detail = format!(
        "{matched} of {} modules in the crawl crate are registered sources; {readers} readers (or the registry), {} descriptors",
        modules.len(),
        registered.len()
    );
    if failures.is_empty() {
        return Ok(Check::holds(8, NAME, detail));
    }
    Ok(Check::violated(8, NAME, detail, failures))
}

fn source_modules() -> Result<BTreeSet<String>> {
    let directory = paths::adapters_dir();
    let listing = fs::read_dir(&directory)
        .with_context(|| format!("listing {}", paths::relative(&directory)))?;
    let mut names: BTreeSet<String> = BTreeSet::new();
    for entry in listing {
        let entry = entry.with_context(|| format!("listing {}", paths::relative(&directory)))?;
        let path = entry.path();
        let Some(name) = path.file_stem().and_then(OsStr::to_str) else {
            continue;
        };
        let is_module =
            path.is_dir() || path.extension().is_some_and(|extension| extension == "rs");
        if is_module && name != "mod" && name != "tests" {
            names.insert(name.to_string());
        }
    }
    Ok(names)
}
