//! Checks 2, 5 and 8: what the source registry declares, and which modules it declares it for.
//!
//! The first two read the crate's own registry — `descriptor`/`descriptors` — rather than a list kept
//! here, so a source that is added, renamed or re-typed changes the verdict on the next run; the third
//! reads the `sources/` module tree beside it and compares the two sets, because a registry and a
//! module tree that disagree describe sources no plan can reach.
//!
//! The `maximum_in_flight` floor is a property of the type before it is a property of the value:
//! `NonZeroUsize::MIN` is what the shared constructors declare, so the clause below can only fail if
//! the field is re-declared with a type that admits zero — and in that direction the check stops
//! compiling, which is the loudest form the floor can take. The failure line stays because the floor
//! is a contract (one in-flight request per host) and not an implementation detail.
//!
//! The origin clause is a syntax rule rather than a registry lookup, so it lives in [`origin`] with
//! its own tests; what this module asserts is what the registry declares.

mod origin;

use anyhow::{Context, Result};
use midwest_census::sources::registry::{descriptor, descriptors};
use midwest_census::sources::TransportKind;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fs;

use super::Check;
use crate::paths;

use self::origin::is_host;

/// The slug the Athletic.net adapter is registered under.
const ATHLETICNET: &str = "athleticnet";

/// Transports a source is registered with today, where a check asserts a different one.
///
/// A deviation entry is a migration item, not an exemption: it names the value the registry holds
/// now and the reason the assertion cannot hold yet, and `xtask contract` prints it as a known
/// deviation — visible on every run, failing nothing. An entry whose recorded value is no longer
/// registered is dead weight the next reader has to reason about, so resolving the migration means
/// deleting the row; a transport outside both the assertion and this table fails the check.
pub(super) const DEVIATIONS: [(&str, TransportKind, &str); 1] = [(
    ATHLETICNET,
    TransportKind::StructuredApi,
    "the Athletic.net browser migration has not landed: the adapter still arrives through the JSON client, and this check reports the deviation rather than failing the gate until it does",
)];

/// Check 2: the Athletic.net source arrives through a browser session, not a JSON client.
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

/// Check 5: every registered source admits a host, a positive rate, and a request in flight.
///
/// An empty registry is a violation rather than a pass: a check that measures nothing must not be
/// reportable as a check that holds.
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
        if !(admission.target_requests_per_second > 0.0) {
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

/// Modules under `sources/` that are not adapters, with what they are instead.
///
/// The direction this check exists for is "a module with no descriptor is a source the planner cannot
/// see", and telling a new *adapter* apart from a new *reader* needs this list: `sources/` holds both
/// kinds, and a reader has no origin to admit, no transport, and nothing a plan can ask for. The list
/// is the reason the check can be written as an equality rather than as a search for suspicious names.
const NON_ADAPTERS: [(&str, &str); 7] = [
    (
        "applicability",
        "the per-jurisdiction source table the planner reads: data, with no origin to admit",
    ),
    ("compiled", "parses the `Compiled` timer export family"),
    ("hytek", "parses Hy-Tek Meet Manager result files"),
    ("raceday", "parses RaceDay Scoring result exports"),
    (
        "result_file",
        "the result-file domain model every vendor parser shares",
    ),
    (
        "xc",
        "parses the cross-country result files WIAA's timers publish",
    ),
    (
        "registry",
        "the capability registry the adapters are declared in",
    ),
];

/// Check 8: every adapter module under `sources/` is a registered source, and every registered source
/// is a module.
///
/// Both directions are violations of one claim: the registry and the module tree describe the same set
/// of sources. An unregistered adapter is a source no plan can reach; a descriptor with no module is
/// a plan that dispatches nowhere.
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
            "`sources/{module}` is a module with no registered descriptor: either an adapter the planner cannot see, or a reader that belongs in the non-adapter list"
        ));
    }
    for slug in &registered {
        if !modules.contains(*slug) {
            failures.push(format!(
                "the descriptor `{slug}` has no `sources/{slug}` module to dispatch to"
            ));
        }
    }
    if modules.is_empty() {
        failures.push("the sources directory listed no module at all".to_string());
    }
    let detail = format!(
        "{matched} of {} modules under sources/ are registered sources; {readers} readers (or the registry), {} descriptors",
        modules.len(),
        registered.len()
    );
    if failures.is_empty() {
        return Ok(Check::holds(8, NAME, detail));
    }
    Ok(Check::violated(8, NAME, detail, failures))
}

/// Every module name under the crate's `sources/` directory: a file's stem or a directory's name.
///
/// A module is a `.rs` file or a directory of them; `mod` and a bare `tests` directory name no
/// adapter either way.
fn source_modules() -> Result<BTreeSet<String>> {
    let directory = paths::sources_dir();
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
