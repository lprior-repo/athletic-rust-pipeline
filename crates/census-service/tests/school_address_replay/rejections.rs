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
    let ccd_acq = replayed
        .overrides()
        .acquisitions
        .iter()
        .find(|acq| acq.provider == "nces-ccd")
        .ok_or("no CCD acquisition")?;
    let path = &ccd_acq.path;
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

#[test]
fn school_address_replay_acquisition_conflict_different_url_same_body() -> TestResult {
    let dir = TempDir::new()?;
    let mut value = manifest(dir.path())?;
    let captures = value["captures"].as_array_mut().ok_or("no captures")?;

    let mut second = captures[2].clone();
    second["url"] =
        serde_json::json!("https://data.nysed.gov/different-profile.php?school=kingston");
    captures.push(second);

    std::fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_vec(&value)?,
    )?;

    let out = dir.path().join("corpus");
    let error = match replay::run(&dir.path().join("manifest.json"), &out) {
        Err(error) => error,
        Ok(_) => return Err("conflicting acquisition tuples accepted".into()),
    };
    match error {
        replay::ReplayError::Refused { detail, .. } => {
            check!(detail.contains("capture acquisition conflict"));
            check!(detail.contains("different provenance tuples"));
        }
        other => return Err(format!("wrong error: {other}").into()),
    }
    Ok(())
}

#[test]
fn school_address_replay_acquisition_conflict_different_date_same_body() -> TestResult {
    let dir = TempDir::new()?;
    let mut value = manifest(dir.path())?;
    let captures = value["captures"].as_array_mut().ok_or("no captures")?;

    let mut second = captures[2].clone();
    second["retrieved_at"] = serde_json::json!("2026-09-30");
    captures.push(second);

    std::fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_vec(&value)?,
    )?;

    let out = dir.path().join("corpus");
    let error = match replay::run(&dir.path().join("manifest.json"), &out) {
        Err(error) => error,
        Ok(_) => return Err("conflicting acquisition tuples accepted".into()),
    };
    match error {
        replay::ReplayError::Refused { detail, .. } => {
            check!(detail.contains("capture acquisition conflict"));
            check!(detail.contains("date"));
        }
        other => return Err(format!("wrong error: {other}").into()),
    }
    Ok(())
}

#[test]
fn school_address_replay_identical_acquisition_tuples_dedup() -> TestResult {
    let dir = TempDir::new()?;
    let mut value = manifest(dir.path())?;
    let captures = value["captures"].as_array_mut().ok_or("no captures")?;

    let second = captures[1].clone();
    captures.push(second);

    std::fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_vec(&value)?,
    )?;

    let out = dir.path().join("corpus");
    let replayed = replay::run(&dir.path().join("manifest.json"), &out)?;

    check!(eq; replayed.overrides().acquisitions.len(), 4);
    Ok(())
}

#[test]
fn school_address_replay_null_date_stays_unknown() -> TestResult {
    let dir = TempDir::new()?;
    let mut value = manifest(dir.path())?;
    let captures = value["captures"].as_array_mut().ok_or("no captures")?;

    captures[1]["retrieved_at"] = serde_json::Value::Null;

    std::fs::write(
        dir.path().join("manifest.json"),
        serde_json::to_vec(&value)?,
    )?;

    let out = dir.path().join("corpus");
    let replayed = replay::run(&dir.path().join("manifest.json"), &out)?;

    let state_ed_acqs = replayed
        .overrides()
        .acquisitions
        .iter()
        .filter(|acq| acq.provider == "state-ed")
        .collect::<Vec<_>>();
    check!(eq; state_ed_acqs.len(), 1);
    check!(eq; state_ed_acqs[0].retrieved_at, None);
    Ok(())
}
