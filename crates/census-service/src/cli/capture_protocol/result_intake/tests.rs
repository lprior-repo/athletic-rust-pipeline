use super::*;
use census_crawl::milesplit::SourceRowLocator;
use census_crawl::participation_evidence::recorded::read_recorded_html_result_capture;
use census_crawl::participation_evidence::{
    RetainedBodyEncodingV1, RetainedHtmlCaptureV1, RetainedHttpProvenanceV1,
};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::Store;
use chrono::{DateTime, Utc};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const FIXTURE_BODY: &[u8] =
    include_bytes!("../../../../../census-crawl/src/participation_evidence/result_ownership/tests/fixtures/milesplit-result.html");
const FIXTURE_HEADERS: &[u8] =
    include_bytes!("../../../../../census-crawl/src/participation_evidence/result_ownership/tests/fixtures/milesplit-result.headers");
const FIXTURE_URL: &str =
    "https://mi.milesplit.com/meets/763888-mhsaa-lp-region-09-1-milford-2026/results/1304896/raw";

pub(super) fn fixture_locators() -> [SourceRowLocator; 3] {
    [
        SourceRowLocator {
            ordinal: 527,
            byte_offset: 80520,
            byte_length: 100,
        },
        SourceRowLocator {
            ordinal: 540,
            byte_offset: 81648,
            byte_length: 92,
        },
        SourceRowLocator {
            ordinal: 572,
            byte_offset: 84465,
            byte_length: 92,
        },
    ]
}

pub(super) fn fixture_capture() -> TestResult<RetainedHtmlCaptureV1> {
    let completed = DateTime::parse_from_rfc3339("2026-10-16T00:06:57Z")?.with_timezone(&Utc);
    Ok(RetainedHtmlCaptureV1::new(
        FIXTURE_URL,
        FIXTURE_URL,
        200,
        completed,
        FIXTURE_BODY,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![
                ("User-Agent".into(), "OpenAI-research/0.1".into()),
                ("Accept".into(), "*/*".into()),
            ],
            response_header_bytes: FIXTURE_HEADERS.to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: DateTime::parse_from_rfc3339("2026-10-16T00:06:38Z")?.with_timezone(&Utc),
        },
    )?)
}

fn fresh_store() -> TestResult<tempfile::TempDir> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    Store::create_current_qualification_v3(&root)?;
    Ok(directory)
}

fn write_capture_file(path: &Path, capture: &RetainedHtmlCaptureV1) -> TestResult {
    let encoded = serde_json::to_vec(capture)?;
    std::fs::write(path, encoded)?;
    Ok(())
}

fn write_rows_file(path: &Path, locators: &[SourceRowLocator]) -> TestResult {
    let encoded = serde_json::to_vec(locators)?;
    std::fs::write(path, encoded)?;
    Ok(())
}

fn intake_root(directory: &tempfile::TempDir) -> PathBuf {
    directory.path().join("store")
}

#[test]
fn genuine_retained_html_result_rows_intake() -> TestResult {
    let directory = fresh_store()?;
    let root = intake_root(&directory);
    let capture = fixture_capture()?;
    let locators = fixture_locators();
    let capture_path = directory.path().join("capture.json");
    let rows_path = directory.path().join("rows.json");
    write_capture_file(&capture_path, &capture)?;
    write_rows_file(&rows_path, &locators)?;
    let mut output = Vec::new();
    run_to(
        &root,
        &ResultIntakeArgs {
            capture: capture_path,
            rows: rows_path,
            operation: "original-html-result-intake".into(),
        },
        &mut output,
    )?;
    let response: serde_json::Value = serde_json::from_slice(&output)?;
    check!(eq; response.get("meet_id").and_then(serde_json::Value::as_str), Some("763888"));
    check!(eq; response.get("result_document_id").and_then(serde_json::Value::as_str), Some("1304896"));
    let context = response
        .get("context_reference")
        .and_then(serde_json::Value::as_str)
        .ok_or("context reference")?;

    let (store, writer) = Store::open_current_qualification_v3(&root)?.into_parts();
    drop(writer);
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let reopened = read_recorded_html_result_capture(&snapshot, context, &locators, &mut grant)?
        .ok_or("persisted HTML result document")?;
    check!(eq; reopened.capture().body_sha256(),
        "95350f1389f9f81fc0354d73637a59cf508413f8bea530a91eb0de7c28cc19e5");
    let persisted = reopened.read(&locators)?;
    check!(eq; persisted.selected_count(), 3);
    check!(eq; persisted.context_reference(), context);
    check!(eq; persisted.original_document().as_bytes(), FIXTURE_BODY);
    Ok(())
}

#[test]
fn same_operation_replay_original_receipt() -> TestResult {
    let directory = fresh_store()?;
    let root = intake_root(&directory);
    let capture = fixture_capture()?;
    let locators = fixture_locators();
    let capture_path = directory.path().join("capture.json");
    let rows_path = directory.path().join("rows.json");
    write_capture_file(&capture_path, &capture)?;
    write_rows_file(&rows_path, &locators)?;

    let mut first_output = Vec::new();
    run_to(
        &root,
        &ResultIntakeArgs {
            capture: capture_path.clone(),
            rows: rows_path.clone(),
            operation: "original-html-result-intake".into(),
        },
        &mut first_output,
    )?;
    let first: serde_json::Value = serde_json::from_slice(&first_output)?;
    let first_digest = first
        .get("receipt_digest")
        .and_then(serde_json::Value::as_str)
        .ok_or("first receipt digest")?;

    let baseline = {
        let (store, writer) = Store::open_current_qualification_v3(&root)?.into_parts();
        drop(writer);
        let digest = store.snapshot().tables_digest(&census_store::Table::ALL)?;
        digest
    };

    let mut replay_output = Vec::new();
    run_to(
        &root,
        &ResultIntakeArgs {
            capture: capture_path,
            rows: rows_path,
            operation: "original-html-result-intake".into(),
        },
        &mut replay_output,
    )?;
    let replay: serde_json::Value = serde_json::from_slice(&replay_output)?;
    let replay_digest = replay
        .get("receipt_digest")
        .and_then(serde_json::Value::as_str)
        .ok_or("replay receipt digest")?;
    check!(eq; first_digest, replay_digest);
    check!(eq; replay.get("repeated").and_then(serde_json::Value::as_bool), Some(true));
    check!(eq; replay.get("appended").and_then(serde_json::Value::as_u64), Some(0));

    let after = {
        let (store, writer) = Store::open_current_qualification_v3(&root)?.into_parts();
        drop(writer);
        let digest = store.snapshot().tables_digest(&census_store::Table::ALL)?;
        digest
    };
    check!(eq; baseline, after);
    Ok(())
}

#[test]
fn foreign_locator_refuses_before_store_open() -> TestResult {
    let directory = tempfile::tempdir()?;
    let capture = fixture_capture()?;
    let foreign_locators = [SourceRowLocator {
        ordinal: 999,
        byte_offset: 1,
        byte_length: 1,
    }];
    let capture_path = directory.path().join("capture.json");
    let rows_path = directory.path().join("rows.json");
    write_capture_file(&capture_path, &capture)?;
    write_rows_file(&rows_path, &foreign_locators)?;
    let root = intake_root(&directory);
    let mut output = Vec::new();
    let error = run_to(
        &root,
        &ResultIntakeArgs {
            capture: capture_path,
            rows: rows_path,
            operation: "foreign-locator-intake".into(),
        },
        &mut output,
    )
    .err()
    .ok_or("foreign locator was accepted")?;
    let error_message = error.to_string();
    check!(
        error_message.contains("ownership")
            || error_message.contains("locator")
            || error_message.contains("row"),
        "refusal reason: {error_message}"
    );
    check!(output.is_empty());
    check!(!root.exists());
    Ok(())
}
