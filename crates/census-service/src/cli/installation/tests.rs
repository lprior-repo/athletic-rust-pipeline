use super::args::InstallationCommand;
use super::error::InstallationCliError;
use super::*;
use crate::cli::profile::error::ProfileCliError;
use crate::cli::profile::input::{self, InputKind};
use census_domain::model::{InstallationConfigV1, ServiceResourceProfileV1};
use census_service::installation::defaults as installation_defaults;
use census_store::Store;
use clap::Parser;
use std::io::{self, Read, Write};

#[path = "../../../tests/installation/fixture.rs"]
mod fixture;
use fixture::{configuration, equal, profile, TestResult};

fn args(values: &[&str]) -> TestResult<InstallationArgs> {
    let cli = crate::cli::Cli::try_parse_from(values)?;
    match cli.command {
        crate::cli::command::Command::Installation(args) => Ok(args),
        other => Err(format!("unexpected command={other:?}").into()),
    }
}

fn operation(command: InstallationCommand) -> InstallationArgs {
    InstallationArgs { command }
}

#[test]
fn defaults_do_not_require_or_open_a_store_and_report_exact_real_contracts() -> TestResult {
    let root = tempfile::tempdir()?;
    let absent = root.path().join("absent");
    let command = args(&["census-service", "installation", "defaults"])?;
    let mut output = Vec::new();
    run_to(Some(&absent), &command, &mut output)?;
    equal(absent.exists(), false)?;
    let parsed: serde_json::Value = serde_json::from_slice(&output)?;
    equal(
        parsed.get("status").and_then(serde_json::Value::as_str),
        Some("defaults"),
    )?;
    let known = census_service::installation::defaults()?;
    equal(
        parsed.get("source_registry_reference"),
        Some(&serde_json::to_value(known.source_registry_reference)?),
    )?;
    equal(
        parsed.get("operating_defaults_reference"),
        Some(&serde_json::to_value(known.operating_defaults_reference)?),
    )?;
    Ok(())
}

#[test]
fn enroll_and_show_require_explicit_existing_store_and_return_json_refusal() -> TestResult {
    for command in [
        operation(InstallationCommand::Show),
        args(&[
            "census-service",
            "installation",
            "enroll",
            "--file",
            "unused.json",
        ])?,
    ] {
        let mut output = Vec::new();
        if !matches!(
            run_to(None, &command, &mut output),
            Err(InstallationCliError::ExplicitStoreRequired)
        ) {
            return Err("implicit store accepted".into());
        }
        let parsed: serde_json::Value = serde_json::from_slice(&output)?;
        equal(
            parsed.get("code").and_then(serde_json::Value::as_str),
            Some("explicit_store_required"),
        )?;
    }
    let dir = tempfile::tempdir()?;
    let absent = dir.path().join("absent");
    let mut output = Vec::new();
    if !matches!(
        run_to(
            Some(&absent),
            &operation(InstallationCommand::Show),
            &mut output
        ),
        Err(InstallationCliError::Store(
            census_store::StoreError::Io { .. }
        ))
    ) {
        return Err("absent store accepted".into());
    }
    equal(absent.exists(), false)
}

#[test]
fn installation_cli_executes_enrolled_repeated_shown_and_conflict_outcomes() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let profile = profile()?;
    store.register_profile(&profile)?;
    drop(store);
    let mut output = Vec::new();
    run_to(
        Some(root.path()),
        &operation(InstallationCommand::Show),
        &mut output,
    )?;
    let shown: serde_json::Value = serde_json::from_slice(&output)?;
    equal(
        shown.get("status").and_then(serde_json::Value::as_str),
        Some("not_enrolled"),
    )?;
    let input_dir = tempfile::tempdir()?;
    let file = input_dir.path().join("installation.json");
    let config = configuration(&profile)?;
    std::fs::write(&file, serde_json::to_vec(&config)?)?;
    let command = operation(InstallationCommand::Enroll(super::args::EnrollArgs {
        file: file.clone(),
    }));
    let mut first = Vec::new();
    run_to(Some(root.path()), &command, &mut first)?;
    let mut repeated = Vec::new();
    run_to(Some(root.path()), &command, &mut repeated)?;
    let first: serde_json::Value = serde_json::from_slice(&first)?;
    let repeated: serde_json::Value = serde_json::from_slice(&repeated)?;
    equal(
        first.get("status").and_then(serde_json::Value::as_str),
        Some("enrolled"),
    )?;
    equal(
        repeated.get("status").and_then(serde_json::Value::as_str),
        Some("repeated"),
    )?;
    equal(first.get("enrollment"), repeated.get("enrollment"))?;
    let mut output = Vec::new();
    run_to(
        Some(root.path()),
        &operation(InstallationCommand::Show),
        &mut output,
    )?;
    let shown: serde_json::Value = serde_json::from_slice(&output)?;
    equal(
        shown.get("status").and_then(serde_json::Value::as_str),
        Some("shown"),
    )?;
    let changed = InstallationConfigV1::new(
        config.initial_profile_id().clone(),
        ServiceResourceProfileV1::new(9, config.service_resources().memory_budget_bytes(), 30)?,
        config.source_registry().clone(),
        config.operating_defaults().clone(),
        config.capability(),
    );
    std::fs::write(&file, serde_json::to_vec(&changed)?)?;
    let mut output = Vec::new();
    if !matches!(
        run_to(Some(root.path()), &command, &mut output),
        Err(InstallationCliError::Configuration(
            census_service::installation::InstallationCompileError::Store(
                census_store::StoreError::Installation(
                    census_store::InstallationError::ImmutableConflict
                )
            )
        ))
    ) {
        return Err("conflicting config accepted".into());
    }
    let refused: serde_json::Value = serde_json::from_slice(&output)?;
    equal(
        refused.get("code").and_then(serde_json::Value::as_str),
        Some("installation_immutable_conflict"),
    )
}

#[test]
fn shared_profile_reader_keeps_existing_profile_semantics_and_bounds_installation() -> TestResult {
    let original_profile = profile()?;
    let input_dir = tempfile::tempdir()?;
    let profile_file = input_dir.path().join("profile.json");
    std::fs::write(&profile_file, serde_json::to_vec(&original_profile)?)?;
    equal(
        input::read_profile(&profile_file)?,
        original_profile.clone(),
    )?;
    let expected = configuration(&original_profile)?;
    let mut bytes = serde_json::to_vec(&expected)?;
    bytes.resize(census_store::MAX_INSTALLATION_RECORD_BYTES, b' ');
    equal(
        input::parse_versioned::<InstallationConfigV1>(&bytes, InputKind::Installation)?,
        expected,
    )?;
    bytes.push(b' ');
    if !matches!(
        input::parse_versioned::<InstallationConfigV1>(&bytes, InputKind::Installation),
        Err(ProfileCliError::InputTooLarge { .. })
    ) {
        return Err("oversized config accepted".into());
    }
    if !matches!(
        input::parse_versioned::<InstallationConfigV1>(
            br#"{"version":2}"#,
            InputKind::Installation
        ),
        Err(ProfileCliError::InputVersion { found: 2, .. })
    ) {
        return Err("future config assumed v1".into());
    }
    if !matches!(
        input::read_file(input_dir.path()),
        Err(ProfileCliError::InputNotFile { .. })
    ) {
        return Err("directory input accepted".into());
    }
    Ok(())
}

struct GrowingReader;
impl Read for GrowingReader {
    fn read(&mut self, target: &mut [u8]) -> io::Result<usize> {
        target.fill(b' ');
        Ok(target.len())
    }
}

#[test]
fn shared_bounded_reader_refuses_growth_and_unknown_installation_input() -> TestResult {
    if !matches!(
        input::read_limited(&mut GrowingReader),
        Err(ProfileCliError::InputTooLarge { .. })
    ) {
        return Err("growing reader escaped bound".into());
    }
    let mut value = serde_json::to_value(configuration(&profile()?)?)?;
    value
        .as_object_mut()
        .ok_or("configuration")?
        .insert("latest".to_string(), serde_json::json!(true));
    if !matches!(
        input::parse_versioned::<InstallationConfigV1>(
            &serde_json::to_vec(&value)?,
            InputKind::Installation
        ),
        Err(ProfileCliError::InputJson { .. })
    ) {
        return Err("unknown input accepted".into());
    }
    Ok(())
}

struct FailedOutput;
impl Write for FailedOutput {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        Err(io::Error::other("fixture output failure"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("fixture output failure"))
    }
}

#[test]
fn installation_output_failure_is_terminal_and_not_reprinted_recursively() -> TestResult {
    match run_to(
        None,
        &operation(InstallationCommand::Defaults),
        &mut FailedOutput,
    ) {
        Err(error) if error.output_failed() => Ok(()),
        other => Err(format!("failed output={other:?}").into()),
    }
}
