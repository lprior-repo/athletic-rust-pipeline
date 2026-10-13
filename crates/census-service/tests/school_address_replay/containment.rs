use std::path::{Path, PathBuf};
use tempfile::TempDir;

use census_service::school_address::replay;

use super::TestResult;

fn write_capture(dir: &Path, name: &str) -> TestResult<PathBuf> {
    let path = dir.join(name);
    std::fs::write(&path, "test capture content")?;
    Ok(path)
}

fn sha256_bytes(path: &Path) -> TestResult<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

#[test]
fn school_address_replay_refuses_absolute_capture_path() -> TestResult {
    let dir = TempDir::new()?;
    let manifest_path = dir.path().join("manifest.json");
    let out = dir.path().join("corpus");

    std::fs::write(
        &manifest_path,
        serde_json::json!({
            "schema_revision": 1,
            "month": "2026-10",
            "captures": [{
                "format": "state-ed-index",
                "path": "/etc/passwd",
                "url": "https://example.com",
                "sha256": "0".repeat(64)
            }]
        })
        .to_string(),
    )?;

    match replay::run(&manifest_path, &out) {
        Err(replay::ReplayError::Refused { detail, .. }) => {
            check!(detail.contains("capture path must be relative"));
        }
        Ok(_) => return Err("a capture outside the source root was accepted".into()),
        Err(_) => return Err("refused for a reason other than containment".into()),
    }
    Ok(())
}

#[test]
fn school_address_replay_refuses_parent_directory_escape() -> TestResult {
    let dir = TempDir::new()?;
    let sub = dir.path().join("sub");
    std::fs::create_dir_all(&sub)?;
    let capture = write_capture(dir.path(), "capture.csv")?;
    let manifest_path = sub.join("manifest.json");
    let out = dir.path().join("corpus");

    std::fs::write(
        &manifest_path,
        serde_json::json!({
            "schema_revision": 1,
            "month": "2026-10",
            "captures": [{
                "format": "state-ed-index",
                "path": "../capture.csv",
                "url": "https://example.com",
                "sha256": sha256_bytes(&capture)?
            }]
        })
        .to_string(),
    )?;

    match replay::run(&manifest_path, &out) {
        Err(replay::ReplayError::Refused { detail, .. }) => {
            check!(detail.contains("capture escapes the declared source root"));
        }
        Ok(_) => return Err("a capture outside the source root was accepted".into()),
        Err(_) => return Err("refused for a reason other than containment".into()),
    }
    Ok(())
}

#[test]
fn school_address_replay_refuses_symlink_escaping_source_root() -> TestResult {
    let dir = TempDir::new()?;
    let root = dir.path().join("root");
    std::fs::create_dir_all(&root)?;
    let manifest_dir = root.join("manifests");
    std::fs::create_dir_all(&manifest_dir)?;
    let outside = write_capture(dir.path(), "outside.csv")?;
    let linked = root.join("linked.csv");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, &linked)?;
    #[cfg(not(unix))]
    {
        std::fs::copy(&outside, &linked)?;
    }
    let manifest_path = manifest_dir.join("manifest.json");
    let out = dir.path().join("corpus");

    std::fs::write(
        &manifest_path,
        serde_json::json!({
            "schema_revision": 1,
            "month": "2026-10",
            "captures": [{
                "format": "state-ed-index",
                "path": "../linked.csv",
                "url": "https://example.com",
                "sha256": sha256_bytes(&outside)?
            }]
        })
        .to_string(),
    )?;

    #[cfg(unix)]
    match replay::run(&manifest_path, &out) {
        Err(replay::ReplayError::Refused { detail, .. }) => {
            check!(detail.contains("capture escapes the declared source root"));
        }
        Ok(_) => return Err("a capture outside the source root was accepted".into()),
        Err(_) => return Err("refused for a reason other than containment".into()),
    }
    #[cfg(not(unix))]
    {
        let _ = replay::run(&manifest_path, &out)?;
    }
    Ok(())
}

fn copy_fixture_root(dir: &TempDir) -> TestResult<PathBuf> {
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
    Ok(root)
}

#[test]
fn school_address_replay_accepts_in_root_capture() -> TestResult {
    let dir = TempDir::new()?;
    let root = copy_fixture_root(&dir)?;
    let out = dir.path().join("corpus");

    let replayed = replay::run(&root.join("manifest.json"), &out)?;
    check!(eq; replayed.overrides().acquisitions().ok_or("replay must retain acquisition records")?.len(), 3);
    Ok(())
}

#[test]
fn school_address_replay_accepts_in_root_symlink() -> TestResult {
    let dir = TempDir::new()?;
    let root = copy_fixture_root(&dir)?;
    #[cfg(unix)]
    {
        std::fs::rename(root.join("state_ed"), root.join("state_ed_real"))?;
        std::os::unix::fs::symlink(root.join("state_ed_real"), root.join("state_ed"))?;
    }
    let out = dir.path().join("corpus");

    let replayed = replay::run(&root.join("manifest.json"), &out)?;
    check!(eq; replayed.overrides().acquisitions().ok_or("replay must retain acquisition records")?.len(), 3);
    Ok(())
}

#[test]
fn school_address_replay_refuses_partial_archive_on_reuse() -> TestResult {
    let dir = TempDir::new()?;
    let root = copy_fixture_root(&dir)?;
    let out = dir.path().join("corpus");

    let replayed = replay::run(&root.join("manifest.json"), &out)?;
    let archive = replayed.archive();
    std::fs::remove_file(archive.join(".complete"))?;

    let store = census_store::Store::open(dir.path().join("store"))?;
    match replayed.apply(&store) {
        Err(replay::ReplayError::Refused { detail, .. }) => {
            check!(detail.contains("incomplete"));
        }
        Ok(_) => return Err("a partial archive was applied".into()),
        Err(_) => return Err("refused for a reason other than the missing marker".into()),
    }
    Ok(())
}

#[test]
fn school_address_replay_provenance_conflict_still_fires() -> TestResult {
    let dir = TempDir::new()?;
    let root = dir.path().join("root");
    std::fs::create_dir_all(&root)?;
    let capture = write_capture(&root, "capture.csv")?;
    let manifest_path = root.join("manifest.json");
    let out = dir.path().join("corpus");
    let capture_sha = sha256_bytes(&capture)?;

    std::fs::write(
        &manifest_path,
        serde_json::json!({
            "schema_revision": 1,
            "month": "2026-10",
            "captures": [
                {
                    "format": "state-ed-index",
                    "path": "capture.csv",
                    "url": "https://example.com/original",
                    "sha256": capture_sha
                },
                {
                    "format": "state-ed-index",
                    "path": "capture.csv",
                    "url": "https://example.com/different",
                    "sha256": capture_sha
                }
            ]
        })
        .to_string(),
    )?;

    match replay::run(&manifest_path, &out) {
        Err(replay::ReplayError::Refused { detail, .. }) => {
            check!(detail.contains("capture acquisition conflict"));
        }
        Ok(_) => return Err("a capture outside the source root was accepted".into()),
        Err(_) => return Err("refused for a reason other than containment".into()),
    }
    Ok(())
}
