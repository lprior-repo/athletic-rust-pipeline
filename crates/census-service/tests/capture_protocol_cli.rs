#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

#[path = "capture_protocol_cli/roster.rs"]
mod roster;

use census_domain::model::{CaptureProtocolVersion, CaptureQualification};
use census_domain::public_identity_qualification::PublicIdentityQualificationV3;
use census_store::Store;
use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn public_capture_init_requires_explicit_root_and_show_never_creates() -> TestResult {
    let cwd = tempfile::tempdir()?;
    let missing = Command::new(env!("CARGO_BIN_EXE_census-service"))
        .current_dir(cwd.path())
        .args(["capture-protocol", "init"])
        .output()?;
    check!(!missing.status.success());
    check!(!cwd.path().join("var").exists());
    let absent = cwd.path().join("absent");
    let shown = Command::new(env!("CARGO_BIN_EXE_census-service"))
        .arg("--store")
        .arg(&absent)
        .args(["capture-protocol", "show"])
        .output()?;
    check!(!shown.status.success());
    check!(!absent.exists());
    Ok(())
}

#[test]
fn public_capture_init_show_and_serve_opt_in_use_the_bound_protocol() -> TestResult {
    let cwd = tempfile::tempdir()?;
    let root = cwd.path().join("fresh");
    let initialized = Command::new(env!("CARGO_BIN_EXE_census-service"))
        .arg("--store")
        .arg(&root)
        .args(["capture-protocol", "init"])
        .output()?;
    check!(
        initialized.status.success(),
        "{}",
        String::from_utf8_lossy(&initialized.stderr)
    );
    let identity: PublicIdentityQualificationV3 = serde_json::from_slice(&initialized.stdout)?;
    let shown = Command::new(env!("CARGO_BIN_EXE_census-service"))
        .arg("--store")
        .arg(&root)
        .args(["capture-protocol", "show"])
        .output()?;
    check!(shown.status.success());
    let record = CaptureQualification::V2(serde_json::from_slice(&shown.stdout)?);
    check!(eq; record.store_identity(), identity.store_identity());
    check!(eq; record.protocol(), CaptureProtocolVersion::V2);
    let refused = Command::new(env!("CARGO_BIN_EXE_census-serve"))
        .arg("--data-dir")
        .arg(&root)
        .output()?;
    check!(!refused.status.success());
    let unknown = Command::new(env!("CARGO_BIN_EXE_census-serve"))
        .args(["--capture-protocol", "fresh-qualification-v2"])
        .output()?;
    check!(!unknown.status.success());
    let preserved = Store::capture_qualification_control_existing(&root)?.show()?;
    check!(eq; record, preserved);
    Ok(())
}

#[test]
fn public_capture_init_refuses_legacy_roots_and_imported_files() -> TestResult {
    let cwd = tempfile::tempdir()?;
    let legacy = cwd.path().join("legacy");
    drop(Store::open(&legacy)?);
    let before = Store::format(&legacy)?;
    let initialized = Command::new(env!("CARGO_BIN_EXE_census-service"))
        .arg("--store")
        .arg(&legacy)
        .args(["capture-protocol", "init"])
        .output()?;
    check!(!initialized.status.success());
    check!(eq; before, Store::format(&legacy)?);
    let imported = cwd.path().join("imported");
    std::fs::create_dir(&imported)?;
    std::fs::write(imported.join("evidence"), b"preserve")?;
    let refused = Command::new(env!("CARGO_BIN_EXE_census-service"))
        .arg("--store")
        .arg(&imported)
        .args(["capture-protocol", "init"])
        .output()?;
    check!(!refused.status.success());
    check!(eq; std::fs::read(imported.join("evidence"))?, b"preserve");
    Ok(())
}

#[test]
fn serve_help_succeeds_without_acquiring_the_supplied_store() -> TestResult {
    let cwd = tempfile::tempdir()?;
    let root = cwd.path().join("owned");
    let store = Store::open_with(&root, 8_388_608)?;
    let before = store.snapshot().sequence();
    let help = Command::new(env!("CARGO_BIN_EXE_census-serve"))
        .current_dir(cwd.path())
        .arg("--data-dir")
        .arg(&root)
        .arg("--help")
        .output()?;
    check!(eq; help.status.code(), Some(0));
    check!(eq; store.snapshot().sequence(), before);
    let invalid = Command::new(env!("CARGO_BIN_EXE_census-serve"))
        .arg("--data-dir")
        .arg(&root)
        .arg("--not-a-supported-flag")
        .output()?;
    check!(eq; invalid.status.code(), Some(1));
    Ok(())
}
