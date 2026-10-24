use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use census_crawl::milesplit::SourceRowLocator;
use census_crawl::participation_evidence::recorded::read_recorded_independent_participation;
use census_crawl::participation_evidence::result_ownership::read_retained_html_result_rows;
use census_crawl::participation_evidence::{
    RetainedBodyEncodingV1, RetainedHtmlCaptureV1, RetainedHttpProvenanceV1, SubjectOwnershipV1,
    PARTICIPATION_EVIDENCE_PHASE,
};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::EventKind;
use census_domain::public_identity_history::RegistryHead;
use census_store::{JournalReadBudget, Table};
use chrono::{DateTime, Utc};
use clap::Parser;
use serde_json::Value;
use std::ffi::OsStr;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type StoreState = (u64, u64, String, RegistryHead);

const STATUS: &[u8] = include_bytes!(
    "../../../../../census-crawl/src/participation_evidence/result_ownership/tests/fixtures/milesplit-result.html"
);
const SCHEDULE: &[u8] = include_bytes!(
    "../../../../../census-crawl/src/participation_evidence/tests/fixtures/dcc-schedule.html"
);
const STATUS_URL: &str =
    "https://mi.milesplit.com/meets/763888-mhsaa-lp-region-09-1-milford-2026/results/1304896/raw";
const SCHEDULE_URL: &str = "https://dccshamrocks.net/sports/track-and-field/schedule/2025-26";

fn capture(
    url: &str,
    body: &[u8],
    headers: &[u8],
    started: &str,
    acquired: &str,
    content_type: &str,
) -> TestResult<RetainedHtmlCaptureV1> {
    Ok(RetainedHtmlCaptureV1::new(
        url,
        url,
        200,
        DateTime::parse_from_rfc3339(acquired)?.with_timezone(&Utc),
        body,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![
                ("User-Agent".into(), "OpenAI-research/0.1".into()),
                ("Accept".into(), "*/*".into()),
            ],
            response_header_bytes: headers.to_vec(),
            content_type: content_type.into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: DateTime::parse_from_rfc3339(started)?.with_timezone(&Utc),
        },
    )?)
}

fn fixture_packet() -> TestResult<IndependentParticipationPacketV1> {
    let status = capture(
        STATUS_URL,
        STATUS,
        include_bytes!("../../../../../census-crawl/src/participation_evidence/result_ownership/tests/fixtures/milesplit-result.headers"),
        "2026-10-16T00:06:38Z",
        "2026-10-16T00:06:57Z",
        "text/html; charset=UTF-8",
    )?;
    let locator = owen_locator(&status)?;
    Ok(IndependentParticipationPacketV1::from_result_row(
        status,
        capture(
            SCHEDULE_URL,
            SCHEDULE,
            include_bytes!("../../../../../census-crawl/src/participation_evidence/tests/fixtures/dcc-schedule.headers"),
            "2026-10-16T00:05:51Z",
            "2026-10-16T00:05:56Z",
            "text/html; charset=utf-8",
        )?,
        locator,
    )?)
}

fn owen_locator(capture: &RetainedHtmlCaptureV1) -> TestResult<SourceRowLocator> {
    let locator = SourceRowLocator {
        ordinal: 208,
        byte_offset: 52466,
        byte_length: 92,
    };
    let document = read_retained_html_result_rows(capture, &[locator])?;
    check!(eq; document.original_document().as_bytes(), STATUS);
    check!(eq; document.published_start(), "2026-05-15");
    let row = document.row(0).ok_or("original Owen400m row")?;
    check!(eq; row.locator(), locator);
    check!(eq; row.event().kind, EventKind::Track400m);
    check!(eq; row.parsed_row().name, "Owen Courser");
    check!(eq; row.parsed_row().school, "Detroit Catholic Central");
    check!(eq; row.parsed_row().grade.map(|grade| grade.get()), Some(11));
    check!(eq; row.original_row()?, "   6 Owen Courser              11 Detroit Catholic Central                    51.27a        ");
    Ok(locator)
}

fn fresh_store() -> TestResult<tempfile::TempDir> {
    let directory = tempfile::tempdir()?;
    Store::create_current_qualification_v3(directory.path().join("store"))?;
    Ok(directory)
}

fn state(root: &Path) -> TestResult<StoreState> {
    let (store, _writer) = Store::open_current_qualification_v3(root)?.into_parts();
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("current identity ledger")?;
    check!(eq; accepted.head().sequence, 0);
    check!(eq; accepted.history_records().len(), 0);
    check!(eq; accepted.public_ids().count(), 0);
    Ok((
        snapshot.sequence(),
        store.receipt_count()?,
        snapshot.tables_digest(&Table::ALL)?,
        accepted.head().clone(),
    ))
}

fn invoke(root: &Path, packet: &Path) -> Result<Value> {
    let cli = crate::cli::Cli::try_parse_from([
        OsStr::new("census-service"),
        OsStr::new("--store"),
        root.as_os_str(),
        OsStr::new("capture-protocol"),
        OsStr::new("participation-intake"),
        OsStr::new("--packet"),
        packet.as_os_str(),
        OsStr::new("--operation"),
        OsStr::new("genuine-independent-participation"),
    ])?;
    let crate::cli::command::Command::CaptureProtocol(args) = &cli.command else {
        anyhow::bail!("participation intake did not parse as capture-protocol");
    };
    let mut output = Vec::new();
    super::super::run_to(cli.store.as_deref(), args, &mut output)?;
    Ok(serde_json::from_slice(&output)?)
}

fn refused(root: &Path, packet: &Path) -> TestResult<anyhow::Error> {
    let before = state(root)?;
    let mut output = Vec::new();
    let error = run_to(
        root,
        &ParticipationIntakeArgs {
            packet: packet.into(),
            operation: "refused-participation".into(),
        },
        &mut output,
    )
    .err()
    .ok_or("invalid participation packet was accepted")?;
    check!(output.is_empty());
    check!(eq; state(root)?, before);
    Ok(error)
}

#[test]
fn genuine_may_fifteen_owen2027_packet_reopens_and_replays_original_receipt() -> TestResult {
    let directory = fresh_store()?;
    let root = directory.path().join("store");
    let before = state(&root)?;
    let packet = fixture_packet()?;
    let reference = packet.reference()?;
    let path = directory.path().join("packet.json");
    std::fs::write(&path, serde_json::to_vec(&packet)?)?;
    let first = invoke(&root, &path)?;
    let (store, writer) = Store::open_current_qualification_v3(&root)?.into_parts();
    let operation = first
        .get("operation")
        .and_then(Value::as_str)
        .ok_or("operation")?;
    let receipt = store.receipt(operation)?.ok_or("durable intake receipt")?;
    check!(eq; first, serde_json::json!({
        "schema": "participation-intake-v1", "status": "intaken_unreviewed",
        "packet_reference": reference.as_str(),
        "capture_references": [reference.as_str(),
            "95350f1389f9f81fc0354d73637a59cf508413f8bea530a91eb0de7c28cc19e5",
            "26833e4c45506da5d776eb844b106dcfb041f67f7916bec0fb59c950b99693eb"],
        "requested_operation": "genuine-independent-participation", "operation": operation,
        "staged_rows": 0, "appended": 0, "repeated": false,
        "receipt_operation": receipt.operation, "receipt_digest": receipt.digest,
        "qualification_scope": "Unreviewed independent participation packet only; no identity, affiliation, calendar, eligibility or current-school acceptance"
    }));
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let retained = read_recorded_independent_participation(&snapshot, &reference, &mut grant)?
        .ok_or("retained independent participation")?;
    let checked = retained.checked();
    check!(eq; checked.reference(), &reference);
    check!(eq; checked.subject().name(), "Owen Courser");
    check!(eq; checked.subject().school(), "Detroit Catholic Central");
    check!(eq; checked.day().as_str(), "2026-05-15");
    check!(eq; checked.school_year().get(), 2025);
    check!(eq; checked.cohort().get(), 2027);
    check!(matches!(
        checked.ownership(),
        SubjectOwnershipV1::DocumentOnly(_)
    ));
    let raw = snapshot
        .journal_payload_raw_bounded(
            PARTICIPATION_EVIDENCE_PHASE,
            reference.as_str(),
            JournalReadBudget::new(recorded::MAX_PROVIDER_SCHOOL_PACKET_BYTES)?,
        )?
        .ok_or("retained packet payload")?;
    let payload: Value = serde_json::from_str(raw.get())?;
    check!(eq; payload, packet.payload()?);
    for (field, expected) in [("status", STATUS), ("schedule", SCHEDULE)] {
        let body = payload
            .get(field)
            .and_then(|capture| capture.get("body_base64"))
            .and_then(Value::as_str)
            .ok_or("retained original body")?;
        check!(eq; STANDARD.decode(body)?, expected);
    }
    drop(retained);
    drop(snapshot);
    drop(store);
    drop(writer);
    let installed = state(&root)?;
    check!(eq; &installed.2, &before.2);
    check!(eq; &installed.3, &before.3);
    check!(eq; installed.1, before.1.checked_add(1).ok_or("receipt count overflow")?);
    let replay = invoke(&root, &path)?;
    let mut expected = first;
    *expected.get_mut("repeated").ok_or("repeated field")? = Value::Bool(true);
    check!(eq; replay, expected);
    check!(eq; state(&root)?, installed);
    Ok(())
}

#[test]
fn malformed_packet_refuses_without_mutation() -> TestResult {
    let directory = fresh_store()?;
    let path = directory.path().join("malformed.json");
    std::fs::write(&path, b"{not-json")?;
    let error = refused(&directory.path().join("store"), &path)?;
    check!(error.downcast_ref::<serde_json::Error>().is_some());
    Ok(())
}

#[test]
fn oversized_packet_refuses_at_the_existing_read_cap_without_mutation() -> TestResult {
    let directory = fresh_store()?;
    let path = directory.path().join("oversized.json");
    let limit = recorded::MAX_PROVIDER_SCHOOL_PACKET_BYTES;
    let requested = limit.checked_add(1).ok_or("packet length overflow")?;
    std::fs::File::create(&path)?.set_len(u64::try_from(requested)?)?;
    let error = refused(&directory.path().join("store"), &path)?;
    check!(matches!(error.downcast_ref::<census_crawl::CrawlError>(),
        Some(census_crawl::CrawlError::Resource { resource: "converter output bytes", requested: actual, limit: available })
        if *actual == requested && *available == limit));
    Ok(())
}

#[test]
fn unsupported_packet_version_refuses_without_a_receipt_or_accepted_effects() -> TestResult {
    let directory = fresh_store()?;
    let path = directory.path().join("unsupported.json");
    let mut payload = fixture_packet()?.payload()?;
    *payload.get_mut("version").ok_or("packet version")? = serde_json::json!(2);
    std::fs::write(&path, serde_json::to_vec(&payload)?)?;
    let error = refused(&directory.path().join("store"), &path)?;
    check!(matches!(error.downcast_ref::<census_crawl::CrawlError>(),
        Some(census_crawl::CrawlError::Store(census_store::StoreError::Refused { detail }))
        if detail == "unsupported independent participation packet version"));
    Ok(())
}
