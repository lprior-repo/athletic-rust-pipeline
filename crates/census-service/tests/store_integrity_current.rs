#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_store::Store;
use std::path::Path;
use std::process::Command;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type Witness = (
    census_domain::model::StoreIdentity,
    Option<u32>,
    u64,
    u64,
    u64,
);

fn command(root: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_census-service"));
    command.arg("--store").arg(root);
    command
}

fn successful(command: &mut Command) -> TestResult<String> {
    let output = command.output()?;
    if !output.status.success() {
        return Err(format!(
            "native command {command:?}: {}",
            String::from_utf8(output.stderr)?
        )
        .into());
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn witness(root: &Path) -> TestResult<Witness> {
    let (store, _writer) = Store::open_current_qualification_v3(root)?.into_parts();
    Ok((
        store.store_identity()?,
        store.store_format()?.lifecycle_revision,
        store.snapshot().sequence(),
        store.evidence_generation(),
        store.derived_generation(),
    ))
}

fn clean_integrity(root: &Path) -> TestResult {
    let stdout = successful(command(root).arg("store-integrity"))?;
    check!(stdout.lines().any(|line| line == "ok\ttrue"));
    check!(stdout.lines().any(|line| line == "http_cache_ok\ttrue"));
    Ok(())
}

#[test]
fn current_integrity_and_cold_restore_preserve_protected_identity() -> TestResult {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("source");
    let backup = directory.path().join("backup");
    let dispatch = directory.path().join("restore-dispatch");
    let restored = directory.path().join("restored");
    successful(command(&source).args(["capture-protocol", "init"]))?;
    successful(command(&source).args(["capture-protocol", "enable-admission"]))?;
    let before = witness(&source)?;
    clean_integrity(&source)?;
    check!(eq; &witness(&source)?, &before);
    check!(Store::open(&source).is_err());
    successful(command(&source).args(["store-backup", "--to"]).arg(&backup))?;
    successful(
        command(&dispatch)
            .args(["store-restore", "--from"])
            .arg(&backup)
            .arg("--to")
            .arg(&restored),
    )?;
    clean_integrity(&restored)?;
    check!(eq; &witness(&restored)?, &before);
    check!(Store::open(&restored).is_err());
    Ok(())
}

#[test]
fn identity_only_integrity_does_not_grant_legacy_execution() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("identity-only");
    let declaration = Store::create_public_identity_qualification_v3(&root)?;
    clean_integrity(&root)?;
    check!(matches!(
        Store::open(&root),
        Err(census_store::StoreError::PublicIdentity(
            census_store::PublicIdentityError::OptInRequired
        ))
    ));
    let (store, _writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    let retained_identity = store.store_identity()?;
    check!(eq; &retained_identity, declaration.store_identity());
    Ok(())
}
