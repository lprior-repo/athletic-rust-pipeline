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
        assert!(Store::restore(&backup, &restored).is_err(), "{relative}");
        assert!(!restored.exists());
        fs::remove_file(backup.join(relative))?;
        fs::rename(&external, backup.join(relative))?;
        Store::restore(&backup, &restored)?;
        assert_eq!(
            fs::read(restored.join("http/capture"))?,
            b"retained evidence"
        );
        assert!(Store::open(&restored)?.integrity()?.ok);
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
        assert!(Store::restore(&input, &restored).is_err(), "{input:?}");
        assert!(!restored.exists());
    }
    Ok(())
}
