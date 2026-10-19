use super::args::{ProfileCommand, SelectionRequest};
use super::error::ProfileCliError;
use super::input::{self, InputKind};
use super::{run_to, ProfileArgs};
use census_domain::model::{ProfileId, ProfileSelectionV1, StoreIdentity};
use census_store::MAX_PROFILE_RECORD_BYTES;
use clap::Parser;
use std::io::{self, Read, Write};

#[path = "../../../tests/profile_cli/fixture.rs"]
mod fixture;
use fixture::{equal, profile, TestResult};

fn args(values: &[&str]) -> TestResult<ProfileArgs> {
    let cli = crate::cli::Cli::try_parse_from(values)?;
    match cli.command {
        crate::cli::command::Command::Profile(args) => Ok(args),
        other => Err(format!("unexpected command {other:?}").into()),
    }
}

#[test]
fn profile_cli_accepts_exactly_the_byte_ceiling_and_refuses_the_next_byte() -> TestResult {
    let expected = profile()?;
    let mut raw = serde_json::to_vec(&expected)?;
    raw.resize(MAX_PROFILE_RECORD_BYTES, b' ');
    equal(input::parse_profile(&raw)?, expected)?;
    raw.push(b' ');
    match input::parse_profile(&raw) {
        Err(ProfileCliError::InputTooLarge { observed, max }) => {
            equal(observed, u64::try_from(raw.len())?)?;
            equal(max, MAX_PROFILE_RECORD_BYTES)
        }
        other => Err(format!("oversized input produced {other:?}").into()),
    }
}

#[test]
fn profile_cli_refuses_future_versions_before_assuming_the_v1_shape() -> TestResult {
    for kind in [InputKind::Profile, InputKind::Selection] {
        match input::parse_versioned::<serde_json::Value>(br#"{"version":2}"#, kind) {
            Err(ProfileCliError::InputVersion { found: 2, .. }) => (),
            other => return Err(format!("future input produced {other:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn profile_cli_refuses_unknown_fields_dates_empty_scopes_and_future_nested_refs() -> TestResult {
    let base = serde_json::to_value(profile()?)?;
    let mut unknown = base.clone();
    unknown
        .as_object_mut()
        .ok_or("fixture object")?
        .insert("hidden".into(), true.into());
    let mut date = base.clone();
    date.as_object_mut()
        .ok_or("fixture object")?
        .insert("as_of".into(), "2026".into());
    let mut empty = base.clone();
    empty
        .as_object_mut()
        .ok_or("fixture object")?
        .insert("jurisdictions".into(), serde_json::json!([]));
    let mut nested = base;
    nested
        .get_mut("source_plan")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("fixture source plan")?
        .insert("version".into(), 2.into());
    for (raw, code) in [
        (unknown, "input_json_invalid"),
        (date, "input_json_invalid"),
        (empty, "profile_invalid"),
        (nested, "input_json_invalid"),
    ] {
        match input::parse_profile(&serde_json::to_vec(&raw)?) {
            Err(error) => equal(error.code(), code)?,
            Ok(_) => return Err(format!("invalid payload accepted: {raw}").into()),
        }
    }
    Ok(())
}

#[test]
fn profile_cli_selection_roundtrip_preserves_historical_identity_spelling() -> TestResult {
    let expected =
        ProfileSelectionV1::new(StoreIdentity::parse(&"A".repeat(64))?, profile()?.id()?);
    let parsed: ProfileSelectionV1 =
        input::parse_versioned(&serde_json::to_vec(&expected)?, InputKind::Selection)?;
    equal(parsed, expected)
}

struct OneByte<'a>(&'a [u8]);

impl Read for OneByte<'_> {
    fn read(&mut self, target: &mut [u8]) -> io::Result<usize> {
        match (self.0.split_first(), target.first_mut()) {
            (Some((first, rest)), Some(output)) => {
                *output = *first;
                self.0 = rest;
                Ok(1)
            }
            _ => Ok(0),
        }
    }
}

#[test]
fn profile_cli_bounded_reader_accepts_short_reads_and_refuses_growth() -> TestResult {
    let raw = vec![b' '; MAX_PROFILE_RECORD_BYTES];
    equal(input::read_limited(&mut OneByte(&raw))?, raw.clone())?;
    let extra = MAX_PROFILE_RECORD_BYTES
        .checked_add(1)
        .ok_or("fixture overflow")?;
    match input::read_limited(&mut io::Cursor::new(vec![b' '; extra])) {
        Err(ProfileCliError::InputTooLarge { .. }) => Ok(()),
        other => Err(format!("growing input produced {other:?}").into()),
    }
}

#[test]
fn profile_cli_checked_ids_and_exclusive_selection_flags_are_enforced() -> TestResult {
    let id = "a".repeat(64);
    let local = args(&["census-service", "profile", "select", "--id", &id])?;
    match local.command {
        ProfileCommand::Select(args) => match args.prepare()? {
            SelectionRequest::LocalId(actual) => equal(actual, ProfileId::parse(&id)?)?,
            _ => return Err("local ID became an external selection".into()),
        },
        _ => return Err("select was not selected".into()),
    }
    for values in [
        vec!["census-service", "profile", "select"],
        vec![
            "census-service",
            "profile",
            "select",
            "--id",
            &id,
            "--selection",
            "selection.json",
        ],
        vec!["census-service", "profile", "show", "--id", "ABC"],
        vec![
            "census-service",
            "profile",
            "select",
            "--id",
            &id,
            "--activate",
        ],
        vec![
            "census-service",
            "profile",
            "show",
            "--id",
            &id,
            "--ingress",
            "http://localhost:18095",
        ],
    ] {
        if crate::cli::Cli::try_parse_from(values).is_ok() {
            return Err("invalid profile flags accepted".into());
        }
    }
    Ok(())
}

#[test]
fn profile_cli_missing_store_has_machine_readable_failure_without_reading_input() -> TestResult {
    let args = args(&[
        "census-service",
        "profile",
        "register",
        "--file",
        "missing.json",
    ])?;
    let mut output = Vec::new();
    match run_to(None, &args, &mut output) {
        Err(ProfileCliError::StoreRequired) => (),
        other => return Err(format!("missing store produced {other:?}").into()),
    }
    let reply: serde_json::Value = serde_json::from_slice(&output)?;
    equal(reply.get("operation"), Some(&serde_json::json!("register")))?;
    equal(reply.get("status"), Some(&serde_json::json!("failed")))?;
    equal(
        reply.get("code"),
        Some(&serde_json::json!("explicit_store_required")),
    )
}

struct FailingWriter {
    calls: usize,
}

impl Write for FailingWriter {
    fn write(&mut self, _bytes: &[u8]) -> io::Result<usize> {
        self.calls = self
            .calls
            .checked_add(1)
            .ok_or_else(|| io::Error::other("fixture count"))?;
        Err(io::Error::other("fixture output unavailable"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::other("fixture output unavailable"))
    }
}

#[test]
fn profile_cli_output_failure_is_not_retried_or_reported_as_success() -> TestResult {
    let args = args(&["census-service", "profile", "show", "--id", &"a".repeat(64)])?;
    let mut writer = FailingWriter { calls: 0 };
    match run_to(None, &args, &mut writer) {
        Err(ProfileCliError::OutputJson(_)) => (),
        other => return Err(format!("output failure produced {other:?}").into()),
    }
    equal(writer.calls, 1)
}

#[test]
fn profile_cli_preserves_explicit_49_51_and_multi_cohort_configuration() -> TestResult {
    use census_domain::model::GradYear;
    use census_domain::UsJurisdiction;
    let baseline = profile()?;
    let mut national = baseline.input().clone();
    national.jurisdictions = UsJurisdiction::CENSUS_SCOPE.to_vec();
    let mut explicit_51 = baseline.input().clone();
    explicit_51.jurisdictions = UsJurisdiction::ALL.to_vec();
    let mut multiple = baseline.input().clone();
    multiple
        .graduation_years
        .push(GradYear::new(2028).ok_or("fixture cohort")?);
    let parsed_national = input::parse_profile(&serde_json::to_vec(&national)?)?;
    let parsed_51 = input::parse_profile(&serde_json::to_vec(&explicit_51)?)?;
    let parsed_multiple = input::parse_profile(&serde_json::to_vec(&multiple)?)?;
    equal(parsed_national.input().jurisdictions.len(), 49)?;
    equal(parsed_51.input().jurisdictions.len(), 51)?;
    equal(parsed_51.admits_jurisdiction(UsJurisdiction::Alaska), true)?;
    equal(parsed_multiple.input().graduation_years.len(), 2)?;
    for value in [parsed_national, parsed_51, parsed_multiple] {
        if value.id()? == baseline.id()? {
            return Err("distinct CLI configuration collapsed into the default profile".into());
        }
    }
    Ok(())
}

#[test]
fn profile_cli_retry_after_lost_output_returns_the_retained_registration() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let file = directory.path().join("profile.json");
    let expected = profile()?;
    census_store::Store::create_public_identity_qualification_v3(&root)?;
    std::fs::write(&file, serde_json::to_vec(&expected)?)?;
    let args = args(&[
        "census-service",
        "profile",
        "register",
        "--file",
        file.to_str().ok_or("fixture path")?,
    ])?;
    let mut unavailable = FailingWriter { calls: 0 };
    match run_to(Some(&root), &args, &mut unavailable) {
        Err(ProfileCliError::OutputJson(_)) => (),
        other => return Err(format!("lost output produced {other:?}").into()),
    }
    equal(unavailable.calls, 1)?;
    let before = {
        let (store, _writer) =
            census_store::Store::open_public_identity_qualification_v3(&root)?.into_parts();
        equal(store.profile(&expected.id()?)?, expected.clone())?;
        store.snapshot().sequence()
    };
    let mut output = Vec::new();
    run_to(Some(&root), &args, &mut output)?;
    let reply: serde_json::Value = serde_json::from_slice(&output)?;
    equal(reply.get("status"), Some(&serde_json::json!("repeated")))?;
    equal(
        reply.get("profile_id"),
        Some(&serde_json::json!(expected.id()?.as_str())),
    )?;
    let (store, _writer) =
        census_store::Store::open_public_identity_qualification_v3(&root)?.into_parts();
    equal(store.profile(&expected.id()?)?, expected)?;
    equal(store.snapshot().sequence(), before)
}
