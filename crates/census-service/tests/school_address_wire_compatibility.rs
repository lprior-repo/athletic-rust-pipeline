#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use census_domain::model::{normalize_name, CanonicalSchool};
use census_domain::UsJurisdiction;
use census_service::school_address::{
    join_generation, preflight_generation, replay, Mode, Overrides,
};
use census_store::{Store, Table};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../census-crawl/tests/fixtures/school_address_replay/manifest.json")
}

fn school() -> CanonicalSchool {
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Alabama,
        "Albertville High School",
        normalize_name("Albertville High School"),
        Some("Albertville"),
    );
    school
}

fn legacy_maps(replayed: &replay::Replay) -> TestResult<Overrides> {
    let records = replayed
        .overrides()
        .acquisitions()
        .ok_or("replay has no acquisition records")?;
    let mut urls = BTreeMap::new();
    let mut dates = BTreeMap::new();
    for record in records {
        let selector = format!("{}@{}", record.provider, record.path);
        urls.insert(selector.clone(), record.url.clone());
        if let Some(date) = &record.retrieved_at {
            dates.insert(selector, date.clone());
        }
    }
    Ok(Overrides::from_source_pairs(urls, dates)?)
}

fn postal_claims(store: &Store) -> TestResult<Vec<census_domain::model::SchoolPostalAddress>> {
    Ok(store
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .flat_map(|school| school.postal_addresses)
        .collect())
}

#[test]
fn old_and_new_retained_evidence_replay_the_same_claim_and_idempotent_outcomes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let corpus = directory.path().join("corpus");
    let replayed = replay::run(&fixture(), &corpus)?;
    let current_path = corpus.join("replay-evidence.json");
    let current_bytes = std::fs::read(&current_path)?;
    let current: Overrides = serde_json::from_slice(&current_bytes)?;
    check!(eq; current.acquisitions().ok_or("current shape lost acquisitions")?.len(), 3);
    let historical_path = directory.path().join("legacy-replay-evidence.json");
    let historical_bytes = serde_json::to_vec_pretty(&legacy_maps(&replayed)?)?;
    std::fs::write(&historical_path, &historical_bytes)?;
    let historical: Overrides = serde_json::from_slice(&std::fs::read(&historical_path)?)?;
    check!(historical.acquisitions().is_none());
    let original_school = school();
    let new_store = Store::open(directory.path().join("new-store"))?;
    let old_store = Store::open(directory.path().join("old-store"))?;
    new_store.append(Table::Schools, &original_school)?;
    old_store.append(Table::Schools, &original_school)?;
    let new_report = join_generation(&new_store, &corpus, None, current, Mode::Apply)?;
    let old_report = join_generation(&old_store, &corpus, None, historical.clone(), Mode::Apply)?;
    check!(eq; new_report.counters.linked, 1);
    check!(eq; old_report.counters.linked, 1);
    check!(eq; serde_json::to_vec(&new_report.lanes)?, serde_json::to_vec(&old_report.lanes)?);
    let new_claims = postal_claims(&new_store)?;
    let old_claims = postal_claims(&old_store)?;
    check!(eq; old_claims, new_claims);
    let claim = old_claims
        .first()
        .ok_or("missing historical postal claim")?;
    check!(eq; claim.capture_sha256(), "e01af083baa57b60f37571f9607aa8ebe9b225712563ec9f10fd8809fab661c8");
    check!(eq; claim.evidence().source.url.as_deref(), Some("https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip"));
    check!(eq; claim.evidence().observed_on.as_str(), "2026-09-28");
    let new_again = replayed.apply(&new_store)?;
    let old_again = join_generation(&old_store, &corpus, None, historical, Mode::Apply)?;
    check!(eq; (new_again.counters.linked, new_again.counters.already_linked), (0, 1));
    check!(eq; (old_again.counters.linked, old_again.counters.already_linked), (0, 1));
    check!(eq; postal_claims(&old_store)?, new_claims);
    check!(eq; std::fs::read(&historical_path)?, historical_bytes);
    check!(eq; std::fs::read(&current_path)?, current_bytes);
    check!(eq; replayed.overrides().acquisitions().ok_or("replay lost acquisition records")?.len(), 3);
    Ok(())
}

#[test]
fn historical_date_only_inputs_retain_missing_evidence_and_preflight_refusal() -> TestResult {
    let directory = tempfile::tempdir()?;
    let corpus = directory.path().join("corpus");
    let replayed = replay::run(&fixture(), &corpus)?;
    let overrides: Overrides = serde_json::from_str(r#"{"dates":{"nces-ccd":"2026-09-28"}}"#)?;
    check!(preflight_generation(&corpus, overrides.clone()).is_err());
    let store = Store::open(directory.path().join("store"))?;
    store.append(Table::Schools, &school())?;
    let report = join_generation(&store, &corpus, None, overrides.clone(), Mode::Apply)?;
    check!(eq; report.counters.linked, 0);
    check!(eq; report.counters.evidence_missing, 1);
    check!(postal_claims(&store)?.is_empty());
    let ccd = report
        .lanes
        .lanes("nces-ccd")
        .first()
        .ok_or("missing CCD lane")?;
    check!(eq; ccd.url, None);
    check!(eq; ccd.observed_on.as_deref(), Some("2026-09-28"));
    let again = join_generation(&store, &corpus, None, overrides, Mode::Apply)?;
    check!(eq; again.counters.linked, 0);
    check!(eq; again.counters.evidence_missing, 1);
    check!(postal_claims(&store)?.is_empty());
    check!(eq; replayed.overrides().acquisitions().ok_or("replay lost acquisition records")?.len(), 3);
    Ok(())
}
