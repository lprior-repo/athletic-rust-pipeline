use std::path::Path;

use census_domain::model::CanonicalSchool;
use census_domain::UsJurisdiction;
use census_service::school_address::replay;
use census_store::{Store, Table};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use super::{fixture, manifest, TestResult};

fn refuse_conflict(input: &Path, out: &Path, attribute: &str) -> TestResult {
    match replay::run(input, out) {
        Err(replay::ReplayError::Refused { detail, .. }) => {
            check!(detail.contains("capture acquisition conflict"));
            check!(detail.contains(attribute));
        }
        Err(other) => return Err(format!("wrong conflict error: {other}").into()),
        Ok(_) => return Err("conflicting capture metadata was accepted".into()),
    }
    check!(eq; out.join("current").exists(), false);
    Ok(())
}

#[test]
fn same_provider_identical_bytes_at_distinct_paths_refuse_changed_url_both_orders() -> TestResult {
    let dir = TempDir::new()?;
    let mut value = manifest(dir.path())?;
    let captures = value
        .get_mut("captures")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("no captures")?;
    let mut distinct = captures.last().cloned().ok_or("no state-ed capture")?;
    let source = distinct
        .get("path")
        .and_then(serde_json::Value::as_str)
        .ok_or("no source path")?;
    std::fs::copy(
        dir.path().join(source),
        dir.path().join("second-acquisition.html"),
    )?;
    distinct["path"] = serde_json::json!("second-acquisition.html");
    distinct["url"] =
        serde_json::json!("https://data.nysed.gov/different-profile.php?school=kingston");
    captures.push(distinct);
    let input = dir.path().join("manifest.json");
    let out = dir.path().join("corpus");
    std::fs::write(&input, serde_json::to_vec(&value)?)?;
    refuse_conflict(&input, &out, "url")?;
    value
        .get_mut("captures")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("no captures")?
        .reverse();
    std::fs::write(&input, serde_json::to_vec(&value)?)?;
    refuse_conflict(&input, &out, "url")
}

#[test]
fn same_provider_identical_bytes_at_distinct_paths_refuse_changed_date_both_orders() -> TestResult {
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
        .ok_or("no source path")?;
    std::fs::copy(
        dir.path().join(source),
        dir.path().join("second-acquisition.csv"),
    )?;
    distinct["path"] = serde_json::json!("second-acquisition.csv");
    distinct["retrieved_at"] = serde_json::json!("2026-09-29");
    captures.push(distinct);
    let input = dir.path().join("manifest.json");
    let out = dir.path().join("corpus");
    std::fs::write(&input, serde_json::to_vec(&value)?)?;
    refuse_conflict(&input, &out, "date")?;
    value
        .get_mut("captures")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("no captures")?
        .reverse();
    std::fs::write(&input, serde_json::to_vec(&value)?)?;
    refuse_conflict(&input, &out, "date")
}

#[test]
fn distinct_provider_metadata_can_share_identical_retained_bytes() -> TestResult {
    let dir = TempDir::new()?;
    let body = concat!(
        "NCESSCH,SCH_NAME,MSTREET1,MCITY,MSTATE,MZIP,MZIP4,PHONE,GSLO,GSHI,CHARTER_TEXT,NAME,CITY,STATE,STREET,ZIP\n",
        "010000500870,Same Payload School,1 Main St,Albertville,AL,35950,,,09,12,No,Same Payload School,Albertville,AL,1 Main St,35950\n",
    );
    std::fs::write(dir.path().join("shared.csv"), body)?;
    let digest = format!("{:x}", Sha256::digest(body.as_bytes()));
    let input = dir.path().join("manifest.json");
    std::fs::write(
        &input,
        serde_json::to_vec(&serde_json::json!({
            "schema_revision": 1,
            "month": "2026-10",
            "captures": [
                {
                    "format": "nces-ccd", "path": "shared.csv", "sha256": digest,
                    "url": "https://nces.ed.gov/ccd/shared.csv", "retrieved_at": "2026-09-28"
                },
                {
                    "format": "state-ed-tabular", "path": "shared.csv", "sha256": digest,
                    "url": "https://data.nysed.gov/shared.csv", "retrieved_at": "2026-09-29"
                }
            ]
        }))?,
    )?;
    let replayed = replay::run(&input, &dir.path().join("corpus"))?;
    let acquisitions = replayed
        .overrides()
        .acquisitions()
        .ok_or("missing acquisition records")?;
    check!(eq; acquisitions.len(), 2);
    let ccd = acquisitions
        .iter()
        .find(|record| record.provider == "nces-ccd")
        .ok_or("missing CCD")?;
    let state = acquisitions
        .iter()
        .find(|record| record.provider == "state-ed")
        .ok_or("missing state-ed")?;
    check!(eq; ccd.path, state.path);
    check!(eq; ccd.url, "https://nces.ed.gov/ccd/shared.csv");
    check!(eq; ccd.retrieved_at.as_deref(), Some("2026-09-28"));
    check!(eq; state.url, "https://data.nysed.gov/shared.csv");
    check!(eq; state.retrieved_at.as_deref(), Some("2026-09-29"));
    check!(replayed.archive().join(".complete").exists());
    check!(eq; std::fs::read(replayed.archive().join(digest))?, body.as_bytes());
    Ok(())
}

#[test]
fn edited_retained_manifest_is_refused_with_complete_marker_and_valid_bodies() -> TestResult {
    let dir = TempDir::new()?;
    let replayed = replay::run(&fixture(), &dir.path().join("corpus"))?;
    let manifest_path = replayed.archive().join("manifest.json");
    let mut retained: serde_json::Value = serde_json::from_slice(&std::fs::read(&manifest_path)?)?;
    retained["month"] = serde_json::json!("2026-11");
    std::fs::write(&manifest_path, serde_json::to_vec(&retained)?)?;
    check!(replayed.archive().join(".complete").exists());
    let store = Store::open(dir.path().join("store"))?;
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Alabama,
        "Albertville High School",
        "albertville high school",
        Some("Albertville"),
    );
    store.append(Table::Schools, &school)?;
    let before = serde_json::to_vec(&store.scan::<CanonicalSchool>(Table::Schools)?)?;
    match replayed.apply(&store) {
        Err(replay::ReplayError::Refused { path, detail }) => {
            check!(eq; path, manifest_path);
            check!(eq; detail, "retained manifest sha256 mismatch");
        }
        Err(other) => return Err(format!("wrong manifest-integrity error: {other}").into()),
        Ok(_) => return Err("changed retained manifest was applied".into()),
    }
    check!(eq; serde_json::to_vec(&store.scan::<CanonicalSchool>(Table::Schools)?)?, before);
    Ok(())
}
