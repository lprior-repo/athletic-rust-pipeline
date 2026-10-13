use super::super::super::{content_digest, CacheMeta};
use super::super::{replay_preserved_cache, write_preserved_cache, write_preserved_capture};
use super::*;
use std::ffi::OsString;
use std::io::ErrorKind;
use std::process::Command;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const CHILD_CASE: &str = "CENSUS_ARCHIVE_CAPACITY_TEST_CASE";
const CHILD_TEST: &str =
    "net::cache::archive::admission::tests::archive_capacity_environment_child";

fn check_configuration_error(error: FetchError) -> TestResult {
    check!(error.to_string().contains(ARCHIVE_CAPACITY_ENV));
    check!(!error.retryable());
    match error {
        FetchError::Cache { source, .. } => {
            check!(eq; source.kind(), ErrorKind::InvalidData);
            Ok(())
        }
        other => Err(format!("expected typed invalid configuration, got {other}").into()),
    }
}

#[test]
fn pure_capacity_parser_accepts_every_positive_u64_boundary() -> TestResult {
    check!(eq; capacity_of(None)?, DEFAULT_ARCHIVE_BYTES);
    check!(eq; capacity_of(Some("1".to_string()))?, 1);
    check!(eq; capacity_of(Some(" 4096 \n".to_string()))?, 4096);
    check!(eq; capacity_of(Some(u64::MAX.to_string()))?, u64::MAX);
    Ok(())
}

#[test]
fn pure_capacity_parser_refuses_explicit_invalid_values() -> TestResult {
    [
        "",
        " \t\n",
        "0",
        " 0 ",
        "-1",
        "twelve",
        "18446744073709551616",
    ]
    .into_iter()
    .try_for_each(|value| match capacity_of(Some(value.to_string())) {
        Err(error) => check_configuration_error(error),
        Ok(capacity) => Err(format!("invalid value {value:?} admitted {capacity} bytes").into()),
    })
}

#[test]
fn explicit_zero_capacity_refuses_before_creating_any_files() -> TestResult {
    let root = tempfile::tempdir()?;
    let witness = root.path().join("retained.body");
    std::fs::write(&witness, b"retained capture")?;
    let archive = root.path().join("archive");
    match ArchiveAdmission::begin_with_capacity(root.path(), &archive, 0) {
        Err(error) => check_configuration_error(error)?,
        Ok(_) => return Err("zero capacity must refuse admission".into()),
    }
    check!(!archive.exists());
    check!(!root.path().join(ARCHIVE_LOCK_NAME).exists());
    check!(eq; std::fs::read(&witness)?, b"retained capture");
    check!(eq; std::fs::read_dir(root.path())?.count(), 1);
    Ok(())
}

#[test]
fn explicit_zero_capacity_refuses_before_opening_a_missing_root() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("not-created");
    match ArchiveAdmission::begin_with_capacity(&root, &root.join("archive"), 0) {
        Err(error) => check_configuration_error(error)?,
        Ok(_) => return Err("zero capacity must refuse admission".into()),
    }
    check!(!root.exists());
    check!(eq; std::fs::read_dir(directory.path())?.count(), 0);
    Ok(())
}

fn metadata(body: &[u8]) -> CacheMeta {
    CacheMeta {
        url: "https://example.test/archive-capacity".to_string(),
        method: "GET".to_string(),
        status: 200,
        content_digest: content_digest(body),
        bytes: body.len(),
        fetched_at: "2026-10-13T00:00:00Z".to_string(),
        ..CacheMeta::default()
    }
}

fn check_invalid_entrypoints() -> TestResult {
    let root = tempfile::tempdir()?;
    let body_path = root.path().join("source.body");
    let meta_path = root.path().join("source.meta.json");
    let body = b"retained source bytes";
    let meta = metadata(body);
    let encoded = serde_json::to_vec(&meta)?;
    std::fs::write(&body_path, body)?;
    std::fs::write(&meta_path, &encoded)?;
    let outcomes = [
        write_preserved_cache(&body_path, &meta_path, body, &meta),
        write_preserved_capture(&body_path, &meta_path, body, &meta).map(|_| ()),
        replay_preserved_cache(&body_path, &meta_path, &meta).map(|_| ()),
    ];
    outcomes.into_iter().try_for_each(|outcome| match outcome {
        Err(error) => check_configuration_error(error),
        Ok(()) => Err("invalid capacity must refuse every archive entrypoint".into()),
    })?;
    check!(!root.path().join("archive").exists());
    check!(!root.path().join(ARCHIVE_LOCK_NAME).exists());
    check!(!meta_path.with_extension("json.lock").exists());
    check!(eq; std::fs::read(&body_path)?, body);
    check!(eq; std::fs::read(&meta_path)?, encoded);
    check!(eq; std::fs::read_dir(root.path())?.count(), 2);
    Ok(())
}

fn check_valid_environment(expected: u64) -> TestResult {
    check!(eq; archive_capacity()?, expected);
    let root = tempfile::tempdir()?;
    let body = b"valid source bytes";
    let meta = metadata(body);
    let body_path = root.path().join("source.body");
    let meta_path = root.path().join("source.meta.json");
    write_preserved_cache(&body_path, &meta_path, body, &meta)?;
    check!(root.path().join("archive/bodies").is_dir());
    check!(eq; std::fs::read(&body_path)?, body);
    check!(eq; replay_preserved_cache(&body_path, &meta_path, &meta)?, Some(body.to_vec()));
    Ok(())
}

#[test]
fn archive_capacity_environment_child() -> TestResult {
    let Some(case) = std::env::var_os(CHILD_CASE) else {
        return Ok(());
    };
    match case.to_str().ok_or("invalid child case")? {
        "unset" => check_valid_environment(DEFAULT_ARCHIVE_BYTES),
        "positive" => check_valid_environment(4096),
        "empty" | "zero" | "negative" | "overflow" | "nonunicode" => check_invalid_entrypoints(),
        other => Err(format!("unknown archive capacity child case {other}").into()),
    }
}

fn run_environment_child(case: &str, value: Option<OsString>) -> TestResult {
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args(["--exact", CHILD_TEST, "--nocapture"])
        .env(CHILD_CASE, case)
        .env_remove(ARCHIVE_CAPACITY_ENV);
    if let Some(value) = value {
        command.env(ARCHIVE_CAPACITY_ENV, value);
    }
    let output = command.output()?;
    check!(
        output.status.success(),
        "case {case}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    check!(std::str::from_utf8(&output.stdout)?.contains("running 1 test"));
    Ok(())
}

#[test]
fn archive_capacity_environment_is_checked_in_isolated_processes() -> TestResult {
    [
        ("unset", None),
        ("positive", Some("4096")),
        ("empty", Some("")),
        ("zero", Some("0")),
        ("negative", Some("-1")),
        ("overflow", Some("18446744073709551616")),
    ]
    .into_iter()
    .try_for_each(|(case, value)| run_environment_child(case, value.map(OsString::from)))
}

#[cfg(unix)]
#[test]
fn nonunicode_capacity_environment_is_refused_in_an_isolated_process() -> TestResult {
    use std::os::unix::ffi::OsStringExt;

    run_environment_child("nonunicode", Some(OsString::from_vec(vec![0xff])))
}
