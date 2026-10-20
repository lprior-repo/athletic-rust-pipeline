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
    value["captures"][0]["path"] = "absent.csv".into();
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
fn school_address_replay_identical_acquisition_tuples_retain_all_records() -> TestResult {
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

    let pss_acqs = replayed
        .overrides()
        .acquisitions()
        .ok_or("replay must retain acquisition records")?
        .iter()
        .filter(|acq| acq.provider == "nces-pss")
        .collect::<Vec<_>>();
    check!(eq; pss_acqs.len(), 1);
    check!(eq; pss_acqs[0].retrieved_at, None);
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
    let acquisitions = replayed
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
fn school_address_replay_conflict_known_vs_null_date_both_orders() -> TestResult {
    use sha2::{Digest, Sha256};
    let dir = TempDir::new()?;
    let root = dir.path().join("root");
    std::fs::create_dir_all(&root)?;
    let body = b"identical school directory payload for two captures";
    let path_a = root.join("first.csv");
    let path_b = root.join("second.csv");
    std::fs::write(&path_a, body)?;
    std::fs::write(&path_b, body)?;
    let digest = format!("{:x}", Sha256::digest(body));

    let manifest_path = root.join("manifest.json");
    let out = dir.path().join("corpus");
    std::fs::write(
        &manifest_path,
        serde_json::json!({
            "schema_revision": 1,
            "month": "2026-10",
            "captures": [
                {
                    "format": "state-ed-index",
                    "path": "first.csv",
                    "url": "https://example.com/source",
                    "retrieved_at": "2026-09-28",
                    "sha256": digest
                },
                {
                    "format": "state-ed-index",
                    "path": "second.csv",
                    "url": "https://example.com/source",
                    "retrieved_at": null,
                    "sha256": digest
                }
            ]
        })
        .to_string(),
    )?;

    let error = match replay::run(&manifest_path, &out) {
        Err(error) => error,
        Ok(_) => return Err("known vs null date accepted".into()),
    };
    match error {
        replay::ReplayError::Refused { detail, .. } => {
            check!(detail.contains("capture acquisition conflict"));
            check!(detail.contains("date"));
        }
        other => return Err(format!("wrong error: {other}").into()),
    }

    std::fs::write(
        &manifest_path,
        serde_json::json!({
            "schema_revision": 1,
            "month": "2026-10",
            "captures": [
                {
                    "format": "state-ed-index",
                    "path": "second.csv",
                    "url": "https://example.com/source",
                    "retrieved_at": null,
                    "sha256": digest
                },
                {
                    "format": "state-ed-index",
                    "path": "first.csv",
                    "url": "https://example.com/source",
                    "retrieved_at": "2026-09-28",
                    "sha256": digest
                }
            ]
        })
        .to_string(),
    )?;

    let error = match replay::run(&manifest_path, &out) {
        Err(error) => error,
        Ok(_) => return Err("null vs known date accepted".into()),
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
fn school_address_replay_conflict_two_known_dates() -> TestResult {
    use sha2::{Digest, Sha256};
    let dir = TempDir::new()?;
    let root = dir.path().join("root");
    std::fs::create_dir_all(&root)?;
    let body = b"identical school directory payload for two captures";
    let path_a = root.join("first.csv");
    let path_b = root.join("second.csv");
    std::fs::write(&path_a, body)?;
    std::fs::write(&path_b, body)?;
    let digest = format!("{:x}", Sha256::digest(body));

    let manifest_path = root.join("manifest.json");
    let out = dir.path().join("corpus");
    std::fs::write(
        &manifest_path,
        serde_json::json!({
            "schema_revision": 1,
            "month": "2026-10",
            "captures": [
                {
                    "format": "state-ed-index",
                    "path": "first.csv",
                    "url": "https://example.com/source",
                    "retrieved_at": "2026-09-28",
                    "sha256": digest
                },
                {
                    "format": "state-ed-index",
                    "path": "second.csv",
                    "url": "https://example.com/source",
                    "retrieved_at": "2026-09-30",
                    "sha256": digest
                }
            ]
        })
        .to_string(),
    )?;

    let error = match replay::run(&manifest_path, &out) {
        Err(error) => error,
        Ok(_) => return Err("two known dates accepted".into()),
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
fn school_address_replay_identical_tuples_accepted() -> TestResult {
    use std::path::Path;
    let dir = TempDir::new()?;
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../census-crawl/tests/fixtures/school_address_replay");
    let root = dir.path().join("root");
    std::fs::create_dir_all(root.join("nces"))?;
    std::fs::create_dir_all(root.join("state_ed"))?;
    std::fs::copy(source.join("manifest.json"), root.join("manifest.json"))?;
    std::fs::copy(
        source.join("nces/ccd_sch_029_2526_head.csv"),
        root.join("nces/ccd_sch_029_2526_head.csv"),
    )?;
    std::fs::copy(
        source.join("nces/pss2324_pu_head.csv"),
        root.join("nces/pss2324_pu_head.csv"),
    )?;
    std::fs::copy(
        source.join("state_ed/profile_kingston.html"),
        root.join("state_ed/profile_kingston.html"),
    )?;

    let body = std::fs::read(root.join("nces/ccd_sch_029_2526_head.csv"))?;
    let copy_path = root.join("nces/ccd_sch_029_2526_head_copy.csv");
    std::fs::write(&copy_path, &body)?;

    let manifest_path = root.join("manifest.json");
    let mut value: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
    let captures = value["captures"].as_array_mut().ok_or("no captures")?;
    let ccd_capture = captures[0].clone();
    let mut copy = ccd_capture;
    copy["path"] = serde_json::json!("nces/ccd_sch_029_2526_head_copy.csv");
    captures.push(copy);
    std::fs::write(&manifest_path, serde_json::to_vec(&value)?)?;

    let out = dir.path().join("corpus");
    let replayed = replay::run(&manifest_path, &out)?;

    check!(eq; replayed.overrides().acquisitions().ok_or("replay must retain acquisition records")?.len(), 4);
    let ccd_acqs = replayed
        .overrides()
        .acquisitions()
        .ok_or("replay must retain acquisition records")?
        .iter()
        .filter(|acq| acq.provider == "nces-ccd")
        .collect::<Vec<_>>();
    check!(eq; ccd_acqs.len(), 2);
    for acq in ccd_acqs {
        check!(eq; acq.url, "https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip");
        check!(eq; acq.retrieved_at.as_deref(), Some("2026-09-28"));
    }
    Ok(())
}

#[test]
fn school_address_replay_distinct_provenance_preserved_through_readback() -> TestResult {
    use std::path::Path;
    let dir = TempDir::new()?;
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../census-crawl/tests/fixtures/school_address_replay");
    let root = dir.path().join("root");
    std::fs::create_dir_all(root.join("nces"))?;
    std::fs::create_dir_all(root.join("state_ed"))?;
    std::fs::copy(source.join("manifest.json"), root.join("manifest.json"))?;
    std::fs::copy(
        source.join("nces/ccd_sch_029_2526_head.csv"),
        root.join("nces/ccd_sch_029_2526_head.csv"),
    )?;
    std::fs::copy(
        source.join("nces/pss2324_pu_head.csv"),
        root.join("nces/pss2324_pu_head.csv"),
    )?;
    std::fs::copy(
        source.join("state_ed/profile_kingston.html"),
        root.join("state_ed/profile_kingston.html"),
    )?;

    let out = dir.path().join("corpus");
    let replayed = replay::run(&root.join("manifest.json"), &out)?;

    check!(eq; replayed.overrides().acquisitions().ok_or("replay must retain acquisition records")?.len(), 3);

    let ccd_acq = replayed
        .overrides()
        .acquisitions()
        .ok_or("replay must retain acquisition records")?
        .iter()
        .find(|acq| acq.provider == "nces-ccd")
        .ok_or("no CCD acquisition")?;
    check!(eq; ccd_acq.url, "https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip");
    check!(eq; ccd_acq.retrieved_at.as_deref(), Some("2026-09-28"));

    let pss_acq = replayed
        .overrides()
        .acquisitions()
        .ok_or("replay must retain acquisition records")?
        .iter()
        .find(|acq| acq.provider == "nces-pss")
        .ok_or("no PSS acquisition")?;
    check!(eq; pss_acq.url, "https://nces.ed.gov/surveys/pss/zip/pss2324_pu_csv.zip");
    check!(eq; pss_acq.retrieved_at.as_deref(), Some("2026-09-28"));

    let state_acq = replayed
        .overrides()
        .acquisitions()
        .ok_or("replay must retain acquisition records")?
        .iter()
        .find(|acq| acq.provider == "state-ed")
        .ok_or("no state-ed acquisition")?;
    check!(eq; state_acq.url, "https://data.nysed.gov/profile.php?instid=800000038718");
    check!(eq; state_acq.retrieved_at, None);

    let store = Store::open(dir.path().join("store"))?;
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Alabama,
        "Albertville High School",
        "albertville high school",
        Some("Albertville"),
    );
    store.append(Table::Schools, &school)?;

    let report = replayed.apply(&store)?;
    check!(eq; (report.counters.linked, report.counters.no_match), (1, 0));

    let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
    let school = schools
        .iter()
        .find(|s| s.name == "Albertville High School")
        .ok_or("school absent")?;
    let claim = school
        .postal_addresses
        .first()
        .ok_or("postal claim absent")?;
    check!(eq; claim.evidence().source.url.as_deref(), Some("https://nces.ed.gov/ccd/data/zip/ccd_sch_029_2526_w_0a_050626.zip"));
    check!(eq; claim.evidence().observed_on.as_str(), "2026-09-28");

    Ok(())
}
