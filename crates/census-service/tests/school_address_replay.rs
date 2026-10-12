#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

#[path = "school_address_replay/rejections.rs"]
mod rejections;

use census_domain::model::{normalize_name, CanonicalSchool};
use census_domain::UsJurisdiction;
use census_service::school_address::{replay, verify_current, JoinError, OutcomeRow};
use census_store::{Store, Table};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../census-crawl/tests/fixtures/school_address_replay/manifest.json")
}

fn manifest(dir: &Path) -> TestResult<serde_json::Value> {
    let mut value: serde_json::Value = serde_json::from_slice(&std::fs::read(fixture())?)?;
    let captures = value["captures"].as_array_mut().ok_or("no captures")?;
    let base = fixture().parent().ok_or("no fixture parent")?.to_path_buf();
    for capture in captures {
        let relative = capture["path"].as_str().ok_or("no capture path")?;
        capture["path"] = serde_json::to_value(base.join(relative))?;
    }
    std::fs::write(dir.join("manifest.json"), serde_json::to_vec(&value)?)?;
    Ok(value)
}

#[test]
fn school_address_replay_preserves_the_same_corpus_bytes_and_capture_provenance() -> TestResult {
    let dir = TempDir::new()?;
    let out = dir.path().join("corpus");
    let first = replay::run(&fixture(), &out)?;
    let before = verify_current(&out)?;
    let second = replay::run(&fixture(), &out)?;
    let after = verify_current(&out)?;
    check!(eq; first.generation_digest(), second.generation_digest());
    for name in [
        "manifest.json",
        "school_directory.json",
        "school_directory.csv",
        "pipeline_report.json",
        "baseline.json",
        "update_ledger.json",
    ] {
        check!(eq; before.artifact(name)?, after.artifact(name)?);
    }
    check!(eq; first.overrides().urls.len(), 3);
    check!(eq; first.overrides().dates.len(), 2);
    check!(eq; first.overrides().dates.values().collect::<Vec<_>>(), vec!["2026-09-28", "2026-09-28"]);
    check!(eq; first.overrides().urls.values().filter(|url| url.starts_with("https://data.nysed.gov/profile.php?")).count(), 1);
    let captures = std::fs::read_dir(out.join("captures"))?.collect::<Result<Vec<_>, _>>()?;
    check!(eq; captures.len(), 1);
    let archive = captures.first().ok_or("no archive")?.path();
    check!(eq; std::fs::read_dir(archive)?.count(), 4);
    Ok(())
}

#[test]
fn school_address_replay_join_binds_claims_to_pinned_bytes_url_and_actual_date() -> TestResult {
    let dir = TempDir::new()?;
    let replayed = replay::run(&fixture(), &dir.path().join("corpus"))?;
    let store = Store::open(dir.path().join("store"))?;
    for (state, name, city) in [
        (
            UsJurisdiction::Alabama,
            "Albertville High School",
            "Albertville",
        ),
        (
            UsJurisdiction::Ohio,
            "Absent Captured High School",
            "Columbus",
        ),
    ] {
        let (school, _) = CanonicalSchool::new(state, name, normalize_name(name), Some(city));
        store.append(Table::Schools, &school)?;
    }
    let report = replayed.apply(&store)?;
    check!(eq; report.counters.scanned, 2);
    check!(eq; (report.counters.linked, report.counters.no_match), (1, 1));
    check!(eq; (report.counters.postal_filled, report.counters.postal_remaining), (1, 1));
    let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
    let school = schools
        .iter()
        .find(|school| school.name == "Albertville High School")
        .ok_or("school absent")?;
    let claim = school
        .postal_addresses
        .first()
        .ok_or("postal claim absent")?;
    check!(eq; claim.capture_sha256(), "e01af083baa57b60f37571f9607aa8ebe9b225712563ec9f10fd8809fab661c8");
    check!(eq; claim.evidence().source.url.as_deref(), Some("https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip"));
    check!(eq; claim.evidence().observed_on.as_str(), "2026-09-28");
    let rows = std::fs::read_to_string(report.outcomes)?;
    let rows = rows
        .lines()
        .map(serde_json::from_str::<OutcomeRow>)
        .collect::<Result<Vec<_>, _>>()?;
    check!(eq; schools.iter().filter(|school| school.state == Some(UsJurisdiction::Alabama) && !school.postal_addresses.is_empty()).count(), 1);
    check!(eq; rows.iter().filter(|row| row.state.as_deref() == Some("OH") && row.postal_gap).count(), 1);
    let again = replayed.apply(&store)?;
    check!(eq; (again.counters.linked, again.counters.already_linked), (0, 1));
    Ok(())
}

#[test]
fn school_address_replay_refuses_a_generation_replaced_after_capture_binding() -> TestResult {
    let dir = TempDir::new()?;
    let out = dir.path().join("corpus");
    let mut value = manifest(dir.path())?;
    let manifest_path = dir.path().join("manifest.json");
    std::fs::write(&manifest_path, serde_json::to_vec(&value)?)?;
    let replayed = replay::run(&manifest_path, &out)?;
    value["captures"][0]["retrieved_at"] = serde_json::Value::Null;
    std::fs::write(&manifest_path, serde_json::to_vec(&value)?)?;
    let replacement = replay::run(&manifest_path, &out)?;
    check!(replacement.generation_digest() != replayed.generation_digest());
    let store = Store::open(dir.path().join("store"))?;
    match replayed.apply(&store) {
        Err(replay::ReplayError::Evidence(JoinError::Invariant { detail })) => {
            check!(eq; detail, "school-address generation changed after the run preflight")
        }
        Err(other) => return Err(format!("wrong error: {other}").into()),
        Ok(_) => return Err("changed generation accepted".into()),
    }
    Ok(())
}
