#![forbid(unsafe_code)]

use census_service::installation::defaults as installation_defaults;

#[path = "installation/acquisition.rs"]
mod acquisition;
#[path = "installation/fixture.rs"]
mod fixture;
#[path = "installation/image.rs"]
mod image;
use census_store::Store;
use fixture::{configuration, equal, profile, TestResult};
use image::logical_image;
use std::path::Path;
use std::process::{Command, Output};

fn run(root: Option<&Path>, arguments: &[&str]) -> TestResult<Output> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_census-service"));
    if let Some(root) = root {
        command.arg("--store").arg(root);
    }
    Ok(command.args(arguments).output()?)
}

fn reply(output: &Output) -> TestResult<serde_json::Value> {
    Ok(serde_json::from_slice(&output.stdout)?)
}

#[test]
fn actual_installation_cli_has_json_defaults_explicit_store_and_bounded_file_refusals() -> TestResult
{
    let defaults = run(None, &["installation", "defaults"])?;
    equal(defaults.status.success(), true)?;
    equal(
        reply(&defaults)?
            .get("status")
            .and_then(serde_json::Value::as_str),
        Some("defaults"),
    )?;
    let missing = run(None, &["installation", "show"])?;
    equal(missing.status.success(), false)?;
    equal(
        reply(&missing)?
            .get("code")
            .and_then(serde_json::Value::as_str),
        Some("explicit_store_required"),
    )?;
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    drop(store);
    let input_dir = tempfile::tempdir()?;
    let file = input_dir.path().join("oversized.json");
    std::fs::write(
        &file,
        vec![
            b' ';
            census_store::MAX_INSTALLATION_RECORD_BYTES
                .checked_add(1)
                .ok_or("input limit")?
        ],
    )?;
    let output = run(
        Some(root.path()),
        &[
            "installation",
            "enroll",
            "--file",
            file.to_str().ok_or("path")?,
        ],
    )?;
    equal(output.status.success(), false)?;
    equal(
        reply(&output)?
            .get("code")
            .and_then(serde_json::Value::as_str),
        Some("input_oversized"),
    )
}

#[test]
fn actual_cli_enrollment_reopens_and_native_serve_refuses_before_binding() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let profile = profile()?;
    store.register_profile(&profile)?;
    let identity = store.store_identity()?;
    drop(store);
    let input_dir = tempfile::tempdir()?;
    let file = input_dir.path().join("installation.json");
    let config = configuration(&profile)?;
    std::fs::write(&file, serde_json::to_vec(&config)?)?;
    for expected in ["enrolled", "repeated"] {
        let output = run(
            Some(root.path()),
            &[
                "installation",
                "enroll",
                "--file",
                file.to_str().ok_or("path")?,
            ],
        )?;
        equal(output.status.success(), true)?;
        equal(
            reply(&output)?
                .get("status")
                .and_then(serde_json::Value::as_str),
            Some(expected),
        )?;
    }
    let shown = run(Some(root.path()), &["installation", "show"])?;
    equal(shown.status.success(), true)?;
    equal(
        reply(&shown)?
            .get("status")
            .and_then(serde_json::Value::as_str),
        Some("shown"),
    )?;
    let reserved = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))?;
    let output = Command::new(env!("CARGO_BIN_EXE_census-serve"))
        .arg("--data-dir")
        .arg(root.path())
        .arg("--listen")
        .arg(reserved.local_addr()?.to_string())
        .arg("--browser-profile")
        .arg(input_dir.path().join("browser"))
        .arg("--browser-executable")
        .arg(input_dir.path().join("never-execute-browser"))
        .output()?;
    equal(output.status.success(), false)?;
    let stderr = String::from_utf8(output.stderr)?;
    if !stderr.contains("enrollment only")
        || !stderr.contains(identity.as_str())
        || !stderr.contains(config.id()?.as_str())
    {
        return Err(format!("native serve returned the wrong refusal: {stderr}").into());
    }
    let control = Store::installation_control_existing(root.path())?;
    match control.show()? {
        census_store::InstallationState::Enrolled {
            enrollment,
            config: held,
        } => {
            equal(enrollment.store_identity(), &identity)?;
            equal(held, config)
        }
        other => Err(format!("native serve changed enrollment={other:?}").into()),
    }
}

#[test]
fn actual_cli_refuses_live_owner_and_unsupported_exact_reference_without_creating_store(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let output = run(Some(root.path()), &["installation", "show"])?;
    equal(output.status.success(), false)?;
    equal(
        reply(&output)?
            .get("code")
            .and_then(serde_json::Value::as_str),
        Some("store_open_failed"),
    )?;
    drop(store);
    let dir = tempfile::tempdir()?;
    let absent = dir.path().join("absent");
    let mut raw = serde_json::to_value(configuration(&profile()?)?)?;
    raw.get_mut("source_registry")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("source")?
        .insert("revision".to_string(), serde_json::json!(2));
    let input = dir.path().join("invalid-ref.json");
    std::fs::write(&input, serde_json::to_vec(&raw)?)?;
    let output = run(
        Some(&absent),
        &[
            "installation",
            "enroll",
            "--file",
            input.to_str().ok_or("path")?,
        ],
    )?;
    equal(output.status.success(), false)?;
    equal(
        reply(&output)?
            .get("code")
            .and_then(serde_json::Value::as_str),
        Some("installation_reference_unsupported"),
    )?;
    equal(absent.exists(), false)
}

#[test]
fn service_cli_rejects_both_semantic_reference_mismatches_without_any_metadata_mutation(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let profile = profile()?;
    store.register_profile(&profile)?;
    drop(store);
    let input_dir = tempfile::tempdir()?;
    let input = input_dir.path().join("mismatch.json");
    let config = configuration(&profile)?;
    let before = logical_image(root.path())?;
    for field in ["source_registry", "operating_defaults"] {
        let mut raw = serde_json::to_value(&config)?;
        raw.get_mut(field)
            .and_then(serde_json::Value::as_object_mut)
            .ok_or("reference")?
            .insert("revision".to_string(), serde_json::json!(2));
        std::fs::write(&input, serde_json::to_vec(&raw)?)?;
        let output = run(
            Some(root.path()),
            &[
                "installation",
                "enroll",
                "--file",
                input.to_str().ok_or("path")?,
            ],
        )?;
        equal(output.status.success(), false)?;
        equal(
            reply(&output)?
                .get("code")
                .and_then(serde_json::Value::as_str),
            Some("installation_reference_unsupported"),
        )?;
        equal(&logical_image(root.path())?, &before)?;
    }
    Ok(())
}

#[test]
fn representative_real_legacy_mutation_and_export_commands_refuse_without_side_effects(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let profile = profile()?;
    store.register_profile(&profile)?;
    drop(store);
    let config = configuration(&profile)?;
    let mut control = Store::installation_control_existing(root.path())?;
    census_service::installation::enroll(&mut control, &config)?;
    drop(control);
    let before = logical_image(root.path())?;
    let output_dir = tempfile::tempdir()?;
    let data = output_dir.path().join("exported");
    let data_argument = data.to_str().ok_or("data path")?;
    for arguments in [
        vec!["consolidate"],
        vec![
            "export-data",
            "--data",
            data_argument,
            "--school-year",
            "2026",
        ],
    ] {
        let output = run(Some(root.path()), &arguments)?;
        equal(output.status.success(), false)?;
        let stderr = String::from_utf8(output.stderr)?;
        if !stderr.contains("enrollment only") || !stderr.contains(config.id()?.as_str()) {
            return Err(format!("legacy command {arguments:?} refusal={stderr}").into());
        }
        equal(&logical_image(root.path())?, &before)?;
        equal(
            std::fs::read_dir(root.path().join("out"))?.next().is_none(),
            true,
        )?;
        equal(data.exists(), false)?;
    }
    Ok(())
}
