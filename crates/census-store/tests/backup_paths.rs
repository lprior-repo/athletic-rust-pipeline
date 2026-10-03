#![cfg(unix)]

use census_store::Store;
use std::fs;
use std::os::unix::fs::symlink;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn restore_refuses_symlinked_manifest_and_payload_components() -> TestResult {
    for relative in ["backup.json", "http", "http/capture"] {
        let root = tempfile::tempdir()?;
        let source = root.path().join("source");
        let backup = root.path().join("backup");
        let restored = root.path().join("restored");
        let external = root.path().join("external");
        let store = Store::open(&source)?;
        fs::write(source.join("http/capture"), b"retained evidence")?;
        drop(store);
        Store::backup(&source, &backup)?;
        fs::rename(backup.join(relative), &external)?;
        symlink(&external, backup.join(relative))?;
        let result = Store::restore(&backup, &restored);
        if !result.is_err() {
            return Err(format!("{relative}: expected refusal, got {result:?}").into());
        }
        if restored.exists() {
            return Err(format!("restore destination exists: {restored:?}").into());
        }
        fs::remove_file(backup.join(relative))?;
        fs::rename(&external, backup.join(relative))?;
        Store::restore(&backup, &restored)?;
        {
            let (left, right) = (
                &fs::read(restored.join("http/capture"))?,
                &b"retained evidence",
            );
            if left != right {
                return Err(format!("left={left:?} right={right:?}").into());
            }
        }
        let report = Store::open(&restored)?.integrity()?;
        if !report.ok {
            return Err(format!("restored integrity: {report:?}").into());
        }
    }
    Ok(())
}

#[test]
fn restore_refuses_a_symlinked_backup_root() -> TestResult {
    let root = tempfile::tempdir()?;
    let source = root.path().join("source");
    let backup = root.path().join("backup");
    let alias = root.path().join("alias");
    let restored = root.path().join("restored");
    drop(Store::open(&source)?);
    Store::backup(&source, &backup)?;
    symlink(&backup, &alias)?;
    for suffix in ["", "/", "/."] {
        let input = std::path::PathBuf::from(format!("{}{suffix}", alias.display()));
        let result = Store::restore(&input, &restored);
        if !result.is_err() {
            return Err(format!("{input:?}: expected refusal, got {result:?}").into());
        }
        if restored.exists() {
            return Err(format!("restore destination exists: {restored:?}").into());
        }
    }
    Ok(())
}
