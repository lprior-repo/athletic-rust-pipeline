#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_domain::model::StoreIdentity;
use census_store::Store;
use std::path::{Path, PathBuf};
use std::process::Command;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type Witness = (StoreIdentity, u64, u64, u64);

fn witness(store: &Store) -> TestResult<Witness> {
    Ok((
        store.store_identity()?,
        store.snapshot().sequence(),
        store.evidence_generation(),
        store.derived_generation(),
    ))
}

fn backup(root: &Path) -> TestResult<(PathBuf, Witness)> {
    let source = root.join("source");
    let backup = root.join("backup");
    Store::create_current_qualification_v3(&source)?;
    let (store, writer) = Store::open_current_qualification_v3(&source)?.into_parts();
    let before = witness(&store)?;
    store.flush()?;
    drop((writer, store));
    let report = Store::backup(&source, &backup)?;
    check!(report.files > 0);
    Ok((backup, before))
}

fn identity(root: &Path) -> TestResult<StoreIdentity> {
    let (store, _writer) = Store::open_current_qualification_v3(root)?.into_parts();
    Ok(store.store_identity()?)
}

fn restore(cwd: &Path, backup: &Path, target: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_census-service"));
    command
        .current_dir(cwd)
        .args(["store-restore", "--from"])
        .arg(backup)
        .arg("--to")
        .arg(target);
    command
}

fn successful(command: &mut Command) -> TestResult {
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "native command {command:?}: {}",
            String::from_utf8(output.stderr)?
        )
        .into());
    }
    check!(String::from_utf8(output.stdout)?.starts_with("restored\tfrom "));
    Ok(())
}

#[test]
fn omitted_store_does_not_open_protected_default_and_repeat_preserves_target() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (backup, source) = backup(directory.path())?;
    let protected = directory.path().join("var/census-service");
    Store::create_current_qualification_v3(&protected)?;
    let (store, _writer) = Store::open_current_qualification_v3(&protected)?.into_parts();
    let before = witness(&store)?;
    let target = directory.path().join("restored");
    successful(&mut restore(directory.path(), &backup, &target))?;
    check!(eq; witness(&store)?, before);
    check!(eq; identity(&target)?, source.0);
    let repeated = restore(directory.path(), &backup, &target).output()?;
    check!(!repeated.status.success());
    let stderr = String::from_utf8(repeated.stderr)?;
    check!(stderr.contains("restoring the store"));
    check!(stderr.contains("is not empty"));
    check!(eq; witness(&store)?, before);
    check!(eq; identity(&target)?, source.0);
    check!(eq; identity(&directory.path().join("source"))?, source.0);
    Ok(())
}

#[test]
fn explicit_unrelated_store_does_not_open_held_owner() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (backup, source) = backup(directory.path())?;
    let protected = directory.path().join("unrelated");
    Store::create_current_qualification_v3(&protected)?;
    let (store, _writer) = Store::open_current_qualification_v3(&protected)?.into_parts();
    let before = witness(&store)?;
    let target = directory.path().join("restored");
    successful(
        restore(directory.path(), &backup, &target)
            .arg("--store")
            .arg(&protected),
    )?;
    check!(eq; witness(&store)?, before);
    check!(eq; identity(&target)?, source.0);
    check!(!directory.path().join("var").exists());
    Ok(())
}

#[test]
fn invalid_backup_reports_restore_error_without_creating_default_or_target() -> TestResult {
    let directory = tempfile::tempdir()?;
    let missing = directory.path().join("missing-backup");
    let target = directory.path().join("restored");
    let output = restore(directory.path(), &missing, &target).output()?;
    check!(!output.status.success());
    check!(String::from_utf8(output.stderr)?.contains("restoring the store"));
    check!(!directory.path().join("var").exists());
    check!(!target.exists());
    Ok(())
}
