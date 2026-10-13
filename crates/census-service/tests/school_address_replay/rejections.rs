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
        .acquisitions()
        .ok_or("replay must retain acquisition records")?
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

    check!(eq; replayed.overrides().acquisitions().ok_or("replay must retain acquisition records")?.len(), 4);
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
        .acquisitions()
        .ok_or("replay must retain acquisition records")?
        .iter()
        .filter(|acq| acq.provider == "state-ed")
        .collect::<Vec<_>>();
    check!(eq; state_ed_acqs.len(), 1);
    check!(eq; state_ed_acqs[0].retrieved_at, None);
    Ok(())
}

#[test]
fn identical_duplicate_acquisitions_preserve_records_and_apply_once() -> TestResult {
    let dir = TempDir::new()?;
    let mut value = manifest(dir.path())?;
    let captures = value
        .get_mut("captures")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("no captures")?;
    let duplicate = captures.first().cloned().ok_or("no CCD capture")?;
    captures.push(duplicate);
    let input = dir.path().join("manifest.json");
    std::fs::write(&input, serde_json::to_vec(&value)?)?;
    let replayed = replay::run(&input, &dir.path().join("corpus"))?;
    let acquisitions = &replayed
        .overrides()
        .acquisitions()
        .ok_or("replay must retain acquisition records")?;
    check!(eq; acquisitions.len(), 4);
    check!(eq; acquisitions.iter().map(|acq| acq.entry).collect::<Vec<_>>(), vec![0, 1, 2, 3]);
    let first = acquisitions.first().ok_or("no first acquisition")?;
    let last = acquisitions.last().ok_or("no duplicate acquisition")?;
    check!(eq; first.provider, last.provider);
    check!(eq; first.path, last.path);
    check!(eq; first.url, last.url);
    check!(eq; first.retrieved_at, last.retrieved_at);
    let store = Store::open(dir.path().join("store"))?;
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Alabama,
        "Albertville High School",
        census_domain::model::normalize_name("Albertville High School"),
        Some("Albertville"),
    );
    store.append(Table::Schools, &school)?;
    let report = replayed.apply(&store)?;
    check!(eq; report.lanes.lanes("nces-ccd").len(), 1);
    check!(eq; report.counters.linked, 1);
    check!(eq; report.counters.refused, 0);
    check!(eq; report.counters.evidence_missing, 0);
    check!(eq; report.counters.postal_filled, 1);
    let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
    let linked = schools.first().ok_or("no linked school")?;
    check!(eq; linked.postal_addresses.len(), 1);
    let claim = linked.postal_addresses.first().ok_or("no postal claim")?;
    check!(eq; claim.capture_sha256(), "e01af083baa57b60f37571f9607aa8ebe9b225712563ec9f10fd8809fab661c8");
    check!(eq; claim.evidence().source.url.as_deref(), Some("https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip"));
    check!(eq; claim.evidence().observed_on.as_str(), "2026-09-28");
    let before = serde_json::to_vec(&schools)?;
    let again = replayed.apply(&store)?;
    check!(eq; again.counters.linked, 0);
    check!(eq; again.counters.already_linked, 1);
    check!(eq; again.counters.refused, 0);
    check!(eq; serde_json::to_vec(&store.scan::<CanonicalSchool>(Table::Schools)?)?, before);
    check!(eq; replayed.overrides().acquisitions().ok_or("replay must retain acquisition records")?.len(), 4);
    Ok(())
}

#[test]
fn different_acquisitions_of_identical_bytes_remain_ambiguous() -> TestResult {
    let dir = TempDir::new()?;
    let mut value = manifest(dir.path())?;
    let captures = value
        .get_mut("captures")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("no captures")?;
    let mut distinct = captures.first().cloned().ok_or("no CCD capture")?;
    let source = distinct
        .get("path")
        .and_then(serde_json::Value::as_str)
        .ok_or("no CCD path")?;
    let copied = dir.path().join("second-acquisition.csv");
    std::fs::copy(source, &copied)?;
    *distinct.get_mut("path").ok_or("no path field")? = serde_json::to_value(&copied)?;
    *distinct.get_mut("retrieved_at").ok_or("no date field")? =
        serde_json::Value::String("2026-09-29".to_string());
    captures.push(distinct);
    let input = dir.path().join("manifest.json");
    std::fs::write(&input, serde_json::to_vec(&value)?)?;
    let replayed = replay::run(&input, &dir.path().join("corpus"))?;
    check!(eq; replayed.overrides().acquisitions().ok_or("replay must retain acquisition records")?.len(), 4);
    let store = Store::open(dir.path().join("store"))?;
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Alabama,
        "Albertville High School",
        census_domain::model::normalize_name("Albertville High School"),
        Some("Albertville"),
    );
    store.append(Table::Schools, &school)?;
    let report = replayed.apply(&store)?;
    let lanes = report.lanes.lanes("nces-ccd");
    check!(eq; lanes.len(), 2);
    check!(eq; lanes.iter().map(|lane| lane.observed_on.as_deref()).collect::<Vec<_>>(), vec![Some("2026-09-28"), Some("2026-09-29")]);
    check!(eq; report.counters.linked, 0);
    check!(eq; report.counters.refused, 1);
    check!(eq; report.counters.postal_filled, 0);
    let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
    check!(eq; schools.first().ok_or("no school")?.postal_addresses.len(), 0);
    let outcomes = std::fs::read_to_string(&report.outcomes)?;
    check!(outcomes.contains("several captures of nces-ccd"));
    let again = replayed.apply(&store)?;
    check!(eq; again.counters.linked, 0);
    check!(eq; again.counters.refused, 1);
    check!(eq; replayed.overrides().acquisitions().ok_or("replay must retain acquisition records")?.len(), 4);
    Ok(())
}

#[test]
fn legacy_selector_artifact_retains_bytes_and_applies_the_same_capture_once() -> TestResult {
    let dir = TempDir::new()?;
    let replayed = replay::run(&super::fixture(), &dir.path().join("corpus"))?;
    let mut urls = std::collections::BTreeMap::new();
    let mut dates = std::collections::BTreeMap::new();
    for acquisition in replayed
        .overrides()
        .acquisitions()
        .ok_or("acquisition form missing")?
    {
        urls.insert(
            format!("{}@{}", acquisition.provider, acquisition.path),
            acquisition.url.clone(),
        );
        if let Some(date) = &acquisition.retrieved_at {
            dates.insert(acquisition.provider.clone(), date.clone());
        }
    }
    let retained = serde_json::to_vec(&serde_json::json!({"urls": urls, "dates": dates}))?;
    let path = dir.path().join("historical-evidence.json");
    std::fs::write(&path, &retained)?;
    let overrides: census_service::school_address::Overrides = serde_json::from_slice(&retained)?;
    check!(overrides.acquisitions().is_none());
    let store = Store::open(dir.path().join("store"))?;
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Alabama,
        "Albertville High School",
        census_domain::model::normalize_name("Albertville High School"),
        Some("Albertville"),
    );
    store.append(Table::Schools, &school)?;
    let report = census_service::school_address::join_generation(
        &store,
        &dir.path().join("corpus"),
        None,
        overrides.clone(),
        census_service::school_address::Mode::Apply,
    )?;
    check!(eq; report.counters.linked, 1);
    check!(eq; report.counters.refused, 0);
    let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
    let claim = schools
        .first()
        .ok_or("linked school missing")?
        .postal_addresses
        .first()
        .ok_or("claim missing")?;
    check!(eq; claim.capture_sha256(), "e01af083baa57b60f37571f9607aa8ebe9b225712563ec9f10fd8809fab661c8");
    check!(eq; claim.evidence().observed_on.as_str(), "2026-09-28");
    check!(eq; claim.evidence().source.url.as_deref(), Some("https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip"));
    let before = serde_json::to_vec(&schools)?;
    let again = census_service::school_address::join_generation(
        &store,
        &dir.path().join("corpus"),
        None,
        overrides,
        census_service::school_address::Mode::Apply,
    )?;
    check!(eq; again.counters.linked, 0);
    check!(eq; again.counters.already_linked, 1);
    check!(eq; serde_json::to_vec(&store.scan::<CanonicalSchool>(Table::Schools)?)?, before);
    check!(eq; std::fs::read(path)?, retained);
    Ok(())
}
