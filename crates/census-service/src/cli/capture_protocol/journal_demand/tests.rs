use census_crawl::milesplit::{RetainedJournalBundle, OWNED_CAPTURE_PHASE, OWNED_MEET_PHASE};
use census_domain::model::{CaptureQualification, StoreIdentity};
use census_store::read::JournalNamespace;
use census_store::{PublicIdentityInputRevisionsV2, ReadBudget, Store, StoreError, Table};
use clap::Parser;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
const GRANT: usize = 4 * 1024 * 1024;

#[derive(Debug, PartialEq, Eq)]
struct State {
    identity: StoreIdentity,
    sequence: u64,
    evidence: u64,
    derived: u64,
    inputs: PublicIdentityInputRevisionsV2,
    receipts: u64,
    tables: String,
}

fn state(root: &Path) -> TestResult<State> {
    let (store, _writer) = Store::open_current_qualification_v3(root)?.into_parts();
    let view = store.snapshot();
    Ok(State {
        identity: view.store_identity()?,
        sequence: view.sequence(),
        evidence: view.revision().evidence_generation(),
        derived: view.revision().derived_generation(),
        inputs: view.public_identity_input_revisions_v2()?,
        receipts: store.receipt_count()?,
        tables: view.tables_digest(&Table::ALL)?,
    })
}

fn current() -> TestResult<tempfile::TempDir> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    Store::create_current_qualification_v3(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    Ok(directory)
}

fn invoke<W: Write>(root: Option<&Path>, output: &mut W) -> anyhow::Result<()> {
    let mut argv = vec![OsString::from("census-service")];
    if let Some(root) = root {
        argv.extend([OsString::from("--store"), root.as_os_str().to_owned()]);
    }
    argv.extend([
        OsString::from("capture-protocol"),
        OsString::from("journal-demand"),
    ]);
    let cli = crate::cli::Cli::try_parse_from(argv)?;
    let crate::cli::command::Command::CaptureProtocol(args) = &cli.command else {
        anyhow::bail!("journal demand did not parse as capture-protocol");
    };
    crate::cli::capture_protocol::run_to(cli.store.as_deref(), args, output)
}

fn expected(root: &Path, rows: &[(&str, &str, Value)]) -> TestResult<Value> {
    let (store, _writer) = Store::open_current_qualification_v3(root)?.into_parts();
    let view = store.snapshot();
    let entries = rows
        .iter()
        .map(|(phase, key, value)| {
            json!({"phase": phase, "key": key, "payload": {
                "presence": "present", "value": value,
                "minimum_successful_read_budget": GRANT
            }})
        })
        .collect::<Vec<_>>();
    let literal = serde_json::to_vec(&json!({
        "version": 1, "store_identity": view.store_identity()?,
        "snapshot_sequence": view.sequence(), "entries": entries
    }))?;
    let bundle: RetainedJournalBundle = serde_json::from_slice(&literal)?;
    let canonical = bundle.encode()?;
    let mut payload_bytes = 0_u64;
    let mut maximum_payload_bytes = 0_u64;
    for entry in &entries {
        let bytes = u64::try_from(serde_json::to_vec(&entry["payload"])?.len())?;
        payload_bytes = payload_bytes
            .checked_add(bytes)
            .ok_or("payload bytes overflow")?;
        maximum_payload_bytes = maximum_payload_bytes.max(bytes);
    }
    let mut phases = Vec::new();
    for phase in [OWNED_CAPTURE_PHASE, OWNED_MEET_PHASE] {
        let physical = view.journal_namespace_fingerprint(
            JournalNamespace::new(phase, "row/")?,
            ReadBudget {
                rows: 16,
                bytes: 1024 * 1024,
                record_bytes: 1024 * 1024,
            },
        )?;
        phases.push(json!({
            "phase": phase,
            "logical_roots": rows.iter().filter(|row| row.0 == phase).count(),
            "physical_rows": physical.physical_rows(),
            "physical_encoded_bytes": physical.encoded_bytes()
        }));
    }
    Ok(json!({
        "schema": "retained-journal-present-demand-v1",
        "demand": {
            "scope": "present_phase_superset", "store_identity": view.store_identity()?,
            "snapshot_sequence": view.sequence(), "phases": phases,
            "present_entries": rows.len(), "recorded_payload_bytes": payload_bytes,
            "maximum_recorded_payload_bytes": maximum_payload_bytes,
            "canonical_bundle_bytes": canonical.len(),
            "canonical_digest": format!("{:x}", Sha256::digest(&canonical)),
            "assumed_read_grant_bytes": GRANT
        },
        "unmeasured": [
            "absent_and_legacy_probes", "original_job_binding", "actual_minimum_read_grants",
            "outer_replay_v4", "attachment_closure", "descriptor_and_cold_decoder_fit",
            "rss_and_workbook_fit"
        ]
    }))
}

#[test]
fn parsed_journal_demand_requires_explicit_root_and_emits_no_bytes() -> TestResult {
    let mut output = Vec::new();
    let error = invoke(None, &mut output)
        .err()
        .ok_or("implicit root was accepted")?;
    check!(eq; error.to_string(), "capture-protocol requires an explicit --store root");
    check!(output.is_empty());
    Ok(())
}

#[test]
fn empty_current_qualification_reports_exact_empty_bundle_without_mutation() -> TestResult {
    let directory = current()?;
    let root = directory.path().join("store");
    let before = state(&root)?;
    check!(matches!(
        before.inputs,
        PublicIdentityInputRevisionsV2::Enabled(_)
    ));
    let mut output = Vec::new();
    invoke(Some(&root), &mut output)?;
    check!(eq; serde_json::from_slice::<Value>(&output)?, expected(&root, &[])?);
    check!(output.ends_with(b"\n"));
    check!(output.len() <= 16 * 1024);
    check!(eq; state(&root)?, before);
    let mut repeated = Vec::new();
    invoke(Some(&root), &mut repeated)?;
    check!(eq; repeated, output);
    check!(eq; state(&root)?, before);
    Ok(())
}

#[test]
fn present_values_null_and_escaped_keys_have_exact_metrics_and_leave_revisions_unchanged(
) -> TestResult {
    let directory = current()?;
    let root = directory.path().join("store");
    let rows = [
        (OWNED_CAPTURE_PHASE, "row/a-null", Value::Null),
        (
            OWNED_CAPTURE_PHASE,
            "row/z-\"quoted",
            json!({"note": "line\nbreak", "value": 9}),
        ),
        (OWNED_MEET_PHASE, "row/meet\\fixture", json!([true, 3])),
    ];
    {
        let (store, _writer) = Store::open_current_qualification_v3(&root)?.into_parts();
        for (phase, key, value) in &rows {
            store.journal_done(phase, key, value)?;
        }
        store.journal_done(
            "unrelated_demand_fixture",
            "row/foreign",
            &json!("excluded"),
        )?;
    }
    let before = state(&root)?;
    let mut output = Vec::new();
    invoke(Some(&root), &mut output)?;
    check!(eq; serde_json::from_slice::<Value>(&output)?, expected(&root, &rows)?);
    check!(eq; state(&root)?, before);
    Ok(())
}

#[test]
fn missing_root_refuses_before_output_and_does_not_create_a_store() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("absent");
    let mut output = Vec::new();
    let error = invoke(Some(&root), &mut output)
        .err()
        .ok_or("missing root was initialized")?;
    let Some(StoreError::Io { path, source }) = error.downcast_ref::<StoreError>() else {
        return Err(format!("expected existing marker I/O refusal, found {error:#}").into());
    };
    check!(path.starts_with(&root));
    check!(eq; path.file_name(), Some(std::ffi::OsStr::new("version")));
    check!(eq; source.kind(), io::ErrorKind::NotFound);
    check!(output.is_empty());
    check!(!root.exists());
    Ok(())
}

#[test]
fn historical_capture_qualification_refuses_without_output_or_upgrade() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("historical");
    let original = Store::create_historical_capture_v1(&root)?;
    let mut output = Vec::new();
    let error = invoke(Some(&root), &mut output)
        .err()
        .ok_or("historical root was measured")?;
    check!(matches!(
        error.downcast_ref::<StoreError>(),
        Some(StoreError::Capture(
            census_store::CaptureError::CaptureProtocolUnsupported
        ))
    ));
    check!(output.is_empty());
    check!(eq; Store::capture_qualification_control_existing(&root)?.show()?, CaptureQualification::V1(original));
    Ok(())
}

#[test]
fn oversized_supported_phase_key_refuses_without_partial_success_json_or_mutation() -> TestResult {
    let directory = current()?;
    let root = directory.path().join("store");
    {
        let (store, _writer) = Store::open_current_qualification_v3(&root)?.into_parts();
        store.journal_done(OWNED_CAPTURE_PHASE, &"k".repeat(513), &Value::Null)?;
    }
    let before = state(&root)?;
    let mut output = Vec::new();
    let error = invoke(Some(&root), &mut output)
        .err()
        .ok_or("513-byte key was measured")?;
    check!(matches!(error.downcast_ref::<StoreError>(),
        Some(StoreError::Refused { detail }) if detail == "unsupported retained source journal key"));
    check!(output.is_empty());
    check!(eq; state(&root)?, before);
    Ok(())
}

struct FailingOutput {
    allow_body: bool,
    bytes: Vec<u8>,
}

impl Write for FailingOutput {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.allow_body && bytes != b"\n" {
            self.bytes.extend_from_slice(bytes);
            Ok(bytes.len())
        } else {
            Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "demand output fixture",
            ))
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
fn body_and_final_newline_output_failures_propagate_without_store_mutation() -> TestResult {
    let directory = current()?;
    let root = directory.path().join("store");
    let before = state(&root)?;
    let mut complete = Vec::new();
    invoke(Some(&root), &mut complete)?;
    for allow_body in [false, true] {
        let mut output = FailingOutput {
            allow_body,
            bytes: Vec::new(),
        };
        let error = invoke(Some(&root), &mut output)
            .err()
            .ok_or("writer error was suppressed")?;
        let source = error
            .downcast_ref::<io::Error>()
            .ok_or("writer cause was lost")?;
        check!(eq; source.kind(), io::ErrorKind::BrokenPipe);
        check!(eq; source.to_string(), "demand output fixture");
        if allow_body {
            check!(eq; error.to_string(), "finishing complete journal demand");
            check!(eq; output.bytes.as_slice(), complete.strip_suffix(b"\n").ok_or("missing newline")?);
        } else {
            check!(eq; error.to_string(), "writing complete journal demand");
            check!(output.bytes.is_empty());
        }
        check!(eq; state(&root)?, before);
    }
    Ok(())
}
