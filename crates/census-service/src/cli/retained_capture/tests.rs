use super::{run_to, CaptureCliError, RetainedCaptureArgs};
use census_crawl::milesplit::{CaptureWitnessError, OWNED_CAPTURE_PHASE, OWNED_FIELDS};
use census_domain::model::serialized_digest;
use census_store::{Store, StoreError};
use clap::Parser;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::{self, Write};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn args(reference: &str) -> RetainedCaptureArgs {
    RetainedCaptureArgs {
        acquisition: reference.into(),
        max_body_bytes: 1024,
    }
}

#[derive(serde::Serialize)]
struct StableCapture<'a> {
    url: String,
    response_url: Option<&'a str>,
    method: &'a str,
    status: u16,
    content_digest: &'a str,
    bytes: usize,
    fetched_at: &'a str,
    content_type: Option<&'a str>,
}

fn fixture(store: &Store) -> Result<String, Box<dyn std::error::Error>> {
    let digest = format!("{:x}", Sha256::digest(b"{}"));
    let metadata = StableCapture {
        url: format!("https://wi.milesplit.com/api/v1/meets/42/performances?isMeetPro=0&fields={OWNED_FIELDS}"),
        response_url: None, method: "GET", status: 200,
        content_digest: &digest, bytes: 2, fetched_at: "2026-10-11T12:00:00Z",
        content_type: Some("application/json"),
    };
    let reference = format!("acquisition/42/{}/manifest", serialized_digest(&metadata)?);
    let mut capture = serde_json::to_value(metadata)?;
    capture
        .as_object_mut()
        .ok_or("capture must be object")?
        .insert("from_cache".into(), false.into());
    let prefix = format!("42/{digest}");
    store.journal_done(
        OWNED_CAPTURE_PHASE,
        &format!("{prefix}/0"),
        &json!({
            "capture": capture, "chunk_index": 0, "raw_base64": "e30="
        }),
    )?;
    store.journal_done(
        OWNED_CAPTURE_PHASE,
        &format!("{prefix}/manifest"),
        &json!({
            "capture": capture, "chunk_bytes": 32768, "encoding": "base64", "chunks": 1
        }),
    )?;
    store.journal_done(
        OWNED_CAPTURE_PHASE,
        &reference,
        &json!({
            "capture": capture, "chunk_bytes": 32768, "encoding": "base64", "chunks": 1,
            "content_manifest": format!("{prefix}/manifest"), "chunk_key_prefix": prefix
        }),
    )?;
    store.flush()?;
    Ok(reference)
}

fn reply(bytes: &[u8]) -> Result<Value, Box<dyn std::error::Error>> {
    Ok(serde_json::from_slice(bytes)?)
}

#[test]
fn retained_capture_cli_requires_explicit_existing_store_without_creating_one() -> TestResult {
    let reference = format!("acquisition/42/{}/manifest", "a".repeat(64));
    let mut output = Vec::new();
    check!(matches!(
        run_to(None, &args(&reference), &mut output),
        Err(CaptureCliError::StoreRequired)
    ));
    check!(eq; reply(&output)?["code"], "explicit_store_required");
    let parent = tempfile::tempdir()?;
    let missing = parent.path().join("absent");
    output.clear();
    check!(run_to(Some(&missing), &args(&reference), &mut output).is_err());
    check!(!missing.exists());
    check!(eq; reply(&output)?["code"], "store_open_failed");
    std::fs::create_dir_all(missing.join("fjall"))?;
    output.clear();
    check!(run_to(Some(&missing), &args(&reference), &mut output).is_err());
    check!(!missing.join("fjall/version").exists());
    check!(!missing.join("out").exists());
    Ok(())
}

#[test]
fn retained_capture_cli_validates_inputs_before_opening_store() -> TestResult {
    let parent = tempfile::tempdir()?;
    let missing = parent.path().join("absent");
    for request in [
        args("../arbitrary"),
        RetainedCaptureArgs {
            acquisition: format!("acquisition/42/{}/manifest", "a".repeat(64)),
            max_body_bytes: 0,
        },
    ] {
        let mut output = Vec::new();
        check!(matches!(
            run_to(Some(&missing), &request, &mut output),
            Err(CaptureCliError::Witness(_))
        ));
        check!(!missing.exists());
        check!(eq; reply(&output)?["status"], "failed");
    }
    Ok(())
}

#[test]
fn retained_capture_cli_reports_occurrence_without_body_or_population_claims() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let reference = fixture(&store)?;
    let before = store.snapshot().sequence();
    drop(store);
    for _ in 0..2 {
        let mut output = Vec::new();
        run_to(Some(root.path()), &args(&reference), &mut output)?;
        let found = reply(&output)?;
        check!(eq; found["status"], "verified");
        check!(eq; found["verified_body_bytes"], 2);
        check!(eq;
            found["capture"]["content_digest"],
            format!("{:x}", Sha256::digest(b"{}"))
        );
        check!(eq; found["capture"]["fetched_at"], "2026-10-11T12:00:00Z");
        check!(eq; found["capture"]["method"], "GET");
        check!(eq; found["capture"]["status"], 200);
        check!(eq; found["capture"]["url"], format!("https://wi.milesplit.com/api/v1/meets/42/performances?isMeetPro=0&fields={OWNED_FIELDS}"));
        check!(eq; found["acquisition"], reference);
        check!(eq; found["snapshot_sequence"], before);
        check!(found["capture"].get("body").is_none());
        check!(found["qualification_scope"]
            .as_str()
            .ok_or("scope missing")?
            .contains("remain unqualified"));
    }
    let store = Store::open(root.path())?;
    check!(eq; store.snapshot().sequence(), before);
    Ok(())
}

#[test]
fn retained_capture_cli_refuses_missing_capture_and_small_grant() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let reference = fixture(&store)?;
    drop(store);
    let mut small = args(&reference);
    small.max_body_bytes = 1;
    let mut output = Vec::new();
    check!(run_to(Some(root.path()), &small, &mut output).is_err());
    check!(eq; reply(&output)?["code"], "capture_budget_exceeded");
    output.clear();
    let missing = args(&format!("acquisition/43/{}/manifest", "a".repeat(64)));
    check!(run_to(Some(root.path()), &missing, &mut output).is_err());
    check!(eq; reply(&output)?["code"], "capture_missing");
    Ok(())
}

#[test]
fn retained_capture_cli_parser_exposes_explicit_grant() -> TestResult {
    let cli = crate::cli::Cli::try_parse_from([
        "census-service",
        "verify-retained-capture",
        "--acquisition",
        "invalid",
        "--max-body-bytes",
        "17",
    ])?;
    match cli.command {
        crate::cli::Command::VerifyRetainedCapture(args) => check!(eq; args.max_body_bytes, 17),
        other => return Err(format!("wrong command: {other:?}").into()),
    }
    Ok(())
}

struct RefuseOutput;

impl Write for RefuseOutput {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::ErrorKind::BrokenPipe.into())
    }
    fn flush(&mut self) -> io::Result<()> {
        Err(io::ErrorKind::BrokenPipe.into())
    }
}

#[test]
fn retained_capture_cli_propagates_output_failure() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let reference = fixture(&store)?;
    drop(store);
    let error = run_to(Some(root.path()), &args(&reference), &mut RefuseOutput)
        .err()
        .ok_or("output unexpectedly succeeded")?;
    check!(error.output_failed());
    Ok(())
}

#[test]
fn retained_capture_cli_preserves_store_source_chain_and_code() -> TestResult {
    let key = "retained-source";
    let witness = CaptureWitnessError::Store {
        key: key.into(),
        source: Box::new(StoreError::Io {
            path: "retained-fixture".into(),
            source: io::Error::new(io::ErrorKind::PermissionDenied, "capture read denied"),
        }),
    };
    match &witness {
        CaptureWitnessError::Store { key: found, .. } => check!(eq; found, key),
        other => return Err(format!("wrong witness: {other:?}").into()),
    }
    let source = std::error::Error::source(&witness).ok_or("store source missing")?;
    let boxed = source
        .downcast_ref::<Box<StoreError>>()
        .ok_or("store source cannot downcast to Box<StoreError>")?;
    let store = boxed.as_ref();
    match store {
        StoreError::Io { path, source } => {
            check!(eq; path, std::path::Path::new("retained-fixture"));
            check!(eq; source.kind(), io::ErrorKind::PermissionDenied);
            check!(eq; source.to_string(), "capture read denied");
        }
        other => return Err(format!("wrong store source: {other:?}").into()),
    }
    let io_source = std::error::Error::source(boxed)
        .ok_or("io source missing")?
        .downcast_ref::<io::Error>()
        .ok_or("io source cannot downcast to io::Error")?;
    check!(eq; io_source.kind(), io::ErrorKind::PermissionDenied);
    let expected = "retained capture entry retained-source could not be read: i/o failed for retained-fixture: capture read denied";
    check!(eq; witness.to_string(), expected);
    let cli = CaptureCliError::from(witness);
    check!(eq; cli.code(), "capture_store_read_failed");
    check!(eq; cli.to_string(), expected);
    check!(matches!(
        std::error::Error::source(&cli)
            .ok_or("cli store source missing")?
            .downcast_ref::<Box<StoreError>>(),
        Some(boxed) if matches!(boxed.as_ref(), StoreError::Io { .. })
    ));
    Ok(())
}
