#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_store::Store;
use std::path::Path;
use std::process::{Command, Output};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn integrity(root: &Path) -> std::io::Result<Output> {
    Command::new(env!("CARGO_BIN_EXE_census-service"))
        .arg("--store")
        .arg(root)
        .arg("store-integrity")
        .output()
}

#[test]
fn missing_store_integrity_refuses_without_creating_a_database() -> TestResult {
    let directory = tempfile::tempdir()?;
    let missing = directory.path().join("absent");
    let output = integrity(&missing)?;
    check!(eq; output.status.code(), Some(1));
    check!(!missing.exists());
    check!(eq; std::fs::read_dir(directory.path())?.count(), 0);
    Ok(())
}

#[test]
fn held_current_owner_refuses_integrity_without_mutating_identity_or_generations() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("current");
    let initialized = Command::new(env!("CARGO_BIN_EXE_census-service"))
        .arg("--store")
        .arg(&root)
        .args(["capture-protocol", "init"])
        .output()?;
    if !initialized.status.success() {
        return Err(String::from_utf8(initialized.stderr)?.into());
    }
    let (store, writer) = Store::open_current_qualification_v3(&root)?.into_parts();
    let before = (
        store.store_identity()?,
        store.snapshot().sequence(),
        store.evidence_generation(),
        store.derived_generation(),
    );
    let output = integrity(&root)?;
    check!(eq; output.status.code(), Some(1));
    let after = (
        store.store_identity()?,
        store.snapshot().sequence(),
        store.evidence_generation(),
        store.derived_generation(),
    );
    check!(eq; &after, &before);
    drop((store, writer));
    let (store, writer) = Store::open_current_qualification_v3(&root)?.into_parts();
    let reopened = (
        store.store_identity()?,
        store.snapshot().sequence(),
        store.evidence_generation(),
        store.derived_generation(),
    );
    check!(eq; &reopened, &before);
    drop((store, writer));
    check!(Store::open(&root).is_err());
    Ok(())
}
