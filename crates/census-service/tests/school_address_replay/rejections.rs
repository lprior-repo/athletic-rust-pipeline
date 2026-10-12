use std::path::PathBuf;

use census_domain::model::CanonicalSchool;
use census_domain::UsJurisdiction;
use census_service::school_address::replay;
use census_store::{Store, Table};
use tempfile::TempDir;

use super::{fixture, manifest, TestResult};
#[test]
fn school_address_replay_refuses_changed_capture_hash_before_publishing() -> TestResult {
    let dir = TempDir::new()?;
    let mut value = manifest(dir.path())?;
    value["captures"][0]["sha256"] = "0".repeat(64).into();
    std::fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_vec(&value)?,
    )?;
    let out = dir.path().join("corpus");
    let error = match replay::run(&dir.path().join("manifest.json"), &out) {
        Err(error) => error,
        Ok(_) => return Err("changed hash accepted".into()),
    };
    match error {
        replay::ReplayError::Refused { detail, .. } => {
            check!(detail.starts_with("sha256 mismatch: expected "))
        }
        other => return Err(format!("wrong error: {other}").into()),
    }
    check!(eq; out.join("current").exists(), false);
    Ok(())
}

#[test]
fn school_address_replay_refuses_a_missing_capture_and_an_unadmitted_format() -> TestResult {
    let dir = TempDir::new()?;
    let mut value = manifest(dir.path())?;
    value["captures"][0]["path"] = dir.path().join("absent.csv").display().to_string().into();
    std::fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_vec(&value)?,
    )?;
    match replay::run(
        &dir.path().join("manifest.json"),
        &dir.path().join("corpus"),
    ) {
        Err(replay::ReplayError::Io { path, source }) => {
            check!(eq; path, dir.path().join("absent.csv"));
            check!(eq; source.kind(), std::io::ErrorKind::NotFound);
        }
        Err(other) => return Err(format!("wrong error: {other}").into()),
        Ok(_) => return Err("missing capture accepted".into()),
    }
    value["captures"][0]["format"] = "guessed-state-export".into();
    std::fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_vec(&value)?,
    )?;
    match replay::run(
        &dir.path().join("manifest.json"),
        &dir.path().join("corpus"),
    ) {
        Err(replay::ReplayError::Json(error)) => {
            check!(error.to_string().contains("unknown variant"))
        }
        Err(other) => return Err(format!("wrong error: {other}").into()),
        Ok(_) => return Err("unknown format accepted".into()),
    }
    Ok(())
}

#[test]
fn school_address_replay_null_historical_date_never_becomes_replay_freshness() -> TestResult {
    let dir = TempDir::new()?;
    let mut value = manifest(dir.path())?;
    value["captures"][0]["retrieved_at"] = serde_json::Value::Null;
    std::fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_vec(&value)?,
    )?;
    let replayed = replay::run(
        &dir.path().join("manifest.json"),
        &dir.path().join("corpus"),
    )?;
    let store = Store::open(dir.path().join("store"))?;
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Alabama,
        "Albertville High School",
        "albertville high school",
        Some("Albertville"),
    );
    store.append(Table::Schools, &school)?;
    let report = replayed.apply(&store)?;
    check!(eq; (report.counters.linked, report.counters.evidence_missing), (0, 1));
    check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?.first().ok_or("no school")?.postal_addresses.len(), 0);
    Ok(())
}
#[test]
fn school_address_replay_refuses_raw_capture_corruption_before_the_join() -> TestResult {
    let dir = TempDir::new()?;
    let out = dir.path().join("corpus");
    let replayed = replay::run(&fixture(), &out)?;
    let selector = replayed
        .overrides()
        .urls
        .keys()
        .find(|key| key.starts_with("nces-ccd@"))
        .ok_or("no CCD selector")?;
    let (_, path) = selector.split_once('@').ok_or("no capture selector path")?;
    std::fs::write(path, b"corrupt retained source")?;
    let store = Store::open(dir.path().join("store"))?;
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Alabama,
        "Albertville High School",
        "albertville high school",
        Some("Albertville"),
    );
    store.append(Table::Schools, &school)?;
    match replayed.apply(&store) {
        Err(replay::ReplayError::Refused {
            path: rejected,
            detail,
        }) => {
            check!(eq; rejected, PathBuf::from(path));
            check!(eq; detail, "retained capture sha256 mismatch");
        }
        Err(other) => return Err(format!("wrong error: {other}").into()),
        Ok(report) => {
            return Err(format!(
                "corrupt retained source was accepted by join: linked={}",
                report.counters.linked
            )
            .into())
        }
    }
    check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?.first().ok_or("no school")?.postal_addresses.len(), 0);
    Ok(())
}
