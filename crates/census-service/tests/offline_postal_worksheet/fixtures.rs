use census_crawl::{hytek, nces, net::Fetcher, wiaa_results, AdapterContext, CaptureContext};
use census_domain::model::{normalize_name, CanonicalSchool, SchoolYear, SourceRef};
use census_domain::UsJurisdiction;
use census_report::{report::Scope, workbook};
use census_service::school_address::replay;
use census_store::{Store, Table};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::{capture_cache, TestResult};

const RESULTS: &str =
    include_str!("../../../census-crawl/tests/fixtures/wiaa_results/d1boysstateresults-dash.htm");
const CCD: &str =
    include_str!("../../../census-crawl/tests/fixtures/nces/ccd_sch_029_2526_head.csv");
pub(super) const RESULT_URL: &str =
    "https://www.wiaawi.org/Portals/0/PDF/Results/Track/2025/d1boysstateresults.htm";
pub(super) const FIXTURE_CACHE_FETCHED_AT: &str = "2026-09-20T00:00:00Z";

fn fixture(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../census-crawl/tests/fixtures")
        .join(path)
}

pub(super) fn seed_source_schools(store: &Store) -> TestResult {
    let parsed = hytek::parse(
        &hytek::lines_from_html(RESULTS),
        SourceRef::new("wiaa_results", Some(RESULT_URL.into())),
    )
    .ok_or("captured WIAA result did not parse")?;
    check!(eq; parsed.date, "2025-06-06");
    check!(eq; parsed.end_date.as_deref(), Some("2025-06-07"));
    let names: BTreeSet<_> = parsed
        .events
        .iter()
        .flat_map(|event| event.rows.iter().map(|row| row.school.as_str()))
        .filter(|name| !name.is_empty())
        .collect();
    for name in names {
        let (school, _) =
            CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name), None);
        store.append(Table::Schools, &school)?;
    }
    let directory = nces::parse_ccd(CCD)?;
    let entry = directory
        .entries()
        .iter()
        .find(|entry| {
            entry
                .name()
                .is_some_and(|name| name.as_str() == "Albertville High School")
        })
        .ok_or("captured Albertville school missing")?;
    let name = entry.name().ok_or("captured school name missing")?.as_str();
    let city = entry
        .address()
        .and_then(|address| address.city())
        .map(|city| city.as_str());
    let (school, _) = CanonicalSchool::new(
        entry
            .jurisdiction()
            .ok_or("captured school jurisdiction missing")?,
        name,
        normalize_name(name),
        city,
    );
    store.append(Table::Schools, &school)?;
    Ok(())
}

pub(super) async fn collect_results(store: &Store) -> TestResult {
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )?
    .with_source("wiaa_results")
    .with_offline(true);
    let archive = format!("<a href=\"{RESULT_URL}\">Boys</a>");
    for (url, _) in wiaa_results::ARCHIVES {
        capture_cache::seed(
            fetcher.cache_dir(),
            url,
            archive.as_bytes(),
            FIXTURE_CACHE_FETCHED_AT,
            &[],
        )?;
    }
    capture_cache::seed(
        fetcher.cache_dir(),
        RESULT_URL,
        RESULTS.as_bytes(),
        FIXTURE_CACHE_FETCHED_AT,
        &[],
    )?;
    let context = AdapterContext {
        capture: CaptureContext::LegacyMetadata,
        fetcher: &fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("fixture year")?,
        observed_on: "2099-01-01".into(),
        performance_as_of: chrono::NaiveDate::parse_from_str("2026-10-07", "%Y-%m-%d")?,
        recording: None,
    };
    let options = wiaa_results::Options {
        limit: None,
        refresh: false,
        observed_on: "2099-01-01".into(),
        seasons: vec![2025],
        states: vec![UsJurisdiction::Wisconsin],
        school_names: Vec::new(),
    };
    let report = wiaa_results::collect(&context, &options).await?;
    check!(eq; (report.errors, report.rows, report.requests, report.from_cache), (0, 1, 0, 5));
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    let before = store.snapshot().tables_digest(&Table::ALL)?;
    let repeated = wiaa_results::collect(&context, &options).await?;
    check!(eq; (repeated.errors, repeated.rows, repeated.requests), (0, 0, 0));
    check!(eq; store.snapshot().tables_digest(&Table::ALL)?, before);
    Ok(())
}

pub(super) fn join_addresses(store: &Store, root: &Path) -> TestResult {
    let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
    let wisconsin = schools
        .iter()
        .filter(|school| school.state == Some(UsJurisdiction::Wisconsin))
        .count();
    let replayed = replay::run(
        &fixture("school_address_replay/manifest.json"),
        &root.join("postal"),
    )?;
    let report = replayed.apply(store)?;
    check!(eq; (report.counters.linked, report.counters.postal_filled), (1, 1));
    check!(eq; report.counters.no_match, u64::try_from(wisconsin)?);
    check!(eq; report.counters.postal_remaining, u64::try_from(wisconsin)?);
    check!(eq; report.counters.refused, 0);
    Ok(())
}

pub(super) fn publish(store: &Store, root: &Path) -> TestResult<PathBuf> {
    Ok(workbook::build(
        store,
        &workbook::Options {
            grad_year: Some(2027),
            out: Some(root.join("publication")),
            limit: None,
            scope: Scope::AllSources,
            jurisdictions: Vec::new(),
            school_year: SchoolYear::new(2026).ok_or("fixture contact year")?,
        },
    )?)
}
