use super::{preserve, publish, ReplayError};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn quarantined(dir: &Path) -> TestResult<Vec<(PathBuf, Vec<u8>)>> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("quarantine filename is not UTF-8")?;
        if !name.contains("recovered-") || name.ends_with(".sidecar") {
            continue;
        }
        found.push((path.clone(), std::fs::read(&path)?));
    }
    found.sort();
    Ok(found)
}

fn temporaries(dir: &Path) -> TestResult<Vec<String>> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let name = entry?
            .file_name()
            .to_str()
            .map(str::to_string)
            .ok_or("temporary filename is not UTF-8")?;
        if name.contains(".tmp.") {
            found.push(name);
        }
    }
    found.sort();
    Ok(found)
}

fn sidecar(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.sidecar", path.display()))
}

#[test]
fn an_occupied_destination_is_quarantined_and_the_correct_bytes_are_installed() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("manifest.json");
    std::fs::write(&target, b"damaged")?;

    preserve(&target, b"correct")?;

    check!(eq; std::fs::read(&target)?, b"correct".to_vec());
    let evidence = quarantined(dir.path())?;
    check!(eq; evidence.len(), 1);
    check!(eq; evidence[0].1, b"damaged".to_vec());
    let note = sidecar(&evidence[0].0);
    check!(note.is_file(), "{} is missing", note.display());
    let text = std::fs::read_to_string(&note)?;
    check!(text.contains("bytes differ after install"), "{text}");
    check!(
        text.contains(&format!("{:x}", Sha256::digest(b"damaged"))),
        "{text}"
    );
    check!(
        text.contains(&format!("{:x}", Sha256::digest(b"correct"))),
        "{text}"
    );
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn a_second_recovery_keeps_its_own_evidence() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("capture.bin");
    std::fs::write(&target, b"first damage")?;
    preserve(&target, b"first correction")?;
    std::fs::write(&target, b"second damage")?;
    preserve(&target, b"second correction")?;

    check!(eq; std::fs::read(&target)?, b"second correction".to_vec());
    let evidence = quarantined(dir.path())?;
    check!(eq; evidence.len(), 2);
    check!(eq; evidence[0].1, b"first damage".to_vec());
    check!(eq; evidence[1].1, b"second damage".to_vec());
    for (path, _) in &evidence {
        let note = sidecar(path);
        check!(note.is_file(), "{} is missing", note.display());
    }
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn an_identical_retry_reuses_the_installed_object() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("capture.bin");

    preserve(&target, b"same bytes")?;
    preserve(&target, b"same bytes")?;

    check!(eq; std::fs::read(&target)?, b"same bytes".to_vec());
    check!(eq; quarantined(dir.path())?.len(), 0);
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn a_non_file_occupant_is_refused_and_left_in_place() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("manifest.json");
    std::fs::create_dir(&target)?;

    match preserve(&target, b"correct") {
        Err(ReplayError::Io { .. } | ReplayError::Refused { .. }) => {}
        other => return Err(format!("a directory occupant was not refused: {other:?}").into()),
    }

    check!(target.is_dir());
    check!(eq; std::fs::read_dir(&target)?.count(), 0);
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn a_derived_report_can_be_replaced_without_quarantine() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("replay-evidence.json");

    publish(&target, b"first run")?;
    check!(eq; std::fs::read(&target)?, b"first run".to_vec());
    publish(&target, b"second run")?;

    check!(eq; std::fs::read(&target)?, b"second run".to_vec());
    check!(eq; quarantined(dir.path())?.len(), 0);
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[test]
fn a_report_cannot_replace_a_directory() -> TestResult {
    let dir = TempDir::new()?;
    let target = dir.path().join("replay-evidence.json");
    std::fs::create_dir(&target)?;

    match publish(&target, b"bytes") {
        Err(ReplayError::Io { .. } | ReplayError::Refused { .. }) => {}
        other => return Err(format!("a directory occupant was not refused: {other:?}").into()),
    }

    check!(target.is_dir());
    check!(eq; temporaries(dir.path())?, Vec::<String>::new());
    Ok(())
}

#[path = "capture/failure_tests.rs"]
mod failures;

#[path = "capture/sync_tests.rs"]
mod sync_failures;

#[path = "capture/concurrency_tests.rs"]
mod concurrency;

#[path = "capture/interruption_tests.rs"]
mod interruption;

#[path = "capture/quarantine_change_tests.rs"]
mod quarantine_changes;
