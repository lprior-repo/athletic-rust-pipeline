use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde_json::json;

use super::super::{CorruptionClass, IntegrityCorruption, IntegrityReport};
use crate::{Store, Table, MAX_JOURNAL_VALUE_BYTES};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const OWNED_PHASE: &str = "milesplit_owned_meet_v2";
const CHUNKED_PHASE: &str = "milesplit_rosters";
const CHUNKED_KEY: &str = "wi:chunked";
const OVERSIZED_CHUNK_BYTES: usize = 600 * 1024;

fn findings(report: &IntegrityReport, class: CorruptionClass) -> Vec<&IntegrityCorruption> {
    report
        .corruptions
        .iter()
        .filter(|finding| finding.class == class)
        .collect()
}

fn details_contain(found: &[&IntegrityCorruption], needle: &str) -> bool {
    found.iter().any(|finding| finding.detail.contains(needle))
}

fn big() -> String {
    "v".repeat(MAX_JOURNAL_VALUE_BYTES + 1)
}

fn chunk_key(phase: &str, key: &str, index: usize) -> TestResult<Vec<u8>> {
    Ok(crate::journal::chunk_key(
        &Store::journal_key(phase, key),
        index,
    )?)
}

fn envelope(key: &str, data: &[u8]) -> TestResult<Vec<u8>> {
    Ok(serde_json::to_vec(&json!({
        "type": "chunk",
        "version": 1,
        "key": key,
        "index": 0,
        "total": 3,
        "data": BASE64_STANDARD.encode(data),
    }))?)
}

fn owned_locator(digest: &str) -> String {
    format!("row/wi/meet/{digest}")
}

#[test]
fn integrity_decodes_a_healthy_chunked_owned_row_without_findings() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let payload = json!({"row": big()});
    let digest = census_domain::model::serialized_digest(&payload)?;
    store.journal_done(OWNED_PHASE, &owned_locator(&digest), &payload)?;
    store.journal_done(CHUNKED_PHASE, "wi:school1", &json!({"athletes": 1}))?;
    let report = store.integrity()?;
    check!(eq; report.corruptions, Vec::<IntegrityCorruption>::new());
    check!(eq; report.decode_limits, Vec::<String>::new());
    check!(report.ok);
    Ok(())
}

#[test]
fn integrity_reports_a_missing_journal_chunk_as_truncated() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.journal_done(CHUNKED_PHASE, CHUNKED_KEY, &big())?;
    store
        .journal
        .remove(chunk_key(CHUNKED_PHASE, CHUNKED_KEY, 1)?)?;
    let report = store.integrity()?;
    check!(!report.ok);
    let truncated = findings(&report, CorruptionClass::Truncated);
    check!(eq; truncated.len(), 1);
    check!(
        details_contain(&truncated, "is missing"),
        "the finding must say which chunk is gone: {:?}",
        truncated.first().map(|finding| finding.detail.as_str())
    );
    check!(
        eq;
        truncated.first().map(|finding| finding.subject.as_str()),
        Some("journal/milesplit_rosters/wi:chunked")
    );
    check!(findings(&report, CorruptionClass::Oversized).is_empty());
    check!(findings(&report, CorruptionClass::DigestMismatch).is_empty());
    Ok(())
}

#[test]
fn integrity_reports_a_short_journal_chunk_as_truncated() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.journal_done(CHUNKED_PHASE, CHUNKED_KEY, &big())?;
    store.journal.insert(
        chunk_key(CHUNKED_PHASE, CHUNKED_KEY, 0)?,
        envelope(CHUNKED_KEY, b"short")?,
    )?;
    let report = store.integrity()?;
    check!(!report.ok);
    let truncated = findings(&report, CorruptionClass::Truncated);
    check!(eq; truncated.len(), 1);
    check!(
        details_contain(&truncated, "against the"),
        "the finding must name the length the manifest records: {:?}",
        truncated.first().map(|finding| finding.detail.as_str())
    );
    Ok(())
}

#[test]
fn integrity_reports_a_chunked_payload_digest_mismatch() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let payload = json!({"row": big()});
    let wrong = "a".repeat(64);
    store.journal_done(OWNED_PHASE, &owned_locator(&wrong), &payload)?;
    let report = store.integrity()?;
    check!(!report.ok);
    let mismatches = findings(&report, CorruptionClass::DigestMismatch);
    check!(eq; mismatches.len(), 1);
    check!(
        details_contain(&mismatches, &wrong) && details_contain(&mismatches, "hashes to"),
        "the finding must name the recorded digest and the computed one: {:?}",
        mismatches.first().map(|finding| finding.detail.as_str())
    );
    check!(findings(&report, CorruptionClass::Truncated).is_empty());
    check!(findings(&report, CorruptionClass::Oversized).is_empty());
    Ok(())
}

#[test]
fn integrity_reports_an_oversized_journal_chunk_as_oversized() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.journal_done(CHUNKED_PHASE, CHUNKED_KEY, &big())?;
    let oversized = vec![b'x'; OVERSIZED_CHUNK_BYTES];
    store.journal.insert(
        chunk_key(CHUNKED_PHASE, CHUNKED_KEY, 0)?,
        envelope(CHUNKED_KEY, &oversized)?,
    )?;
    let report = store.integrity()?;
    check!(!report.ok);
    let oversized_findings = findings(&report, CorruptionClass::Oversized);
    check!(eq; oversized_findings.len(), 1);
    check!(
        details_contain(&oversized_findings, "chunk decode budget"),
        "the finding must name the budget it crossed: {:?}",
        oversized_findings
            .first()
            .map(|finding| finding.detail.as_str())
    );
    check!(findings(&report, CorruptionClass::Truncated).is_empty());
    check!(findings(&report, CorruptionClass::DigestMismatch).is_empty());
    Ok(())
}

#[test]
fn integrity_reports_an_undecodable_journal_chunk() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.journal_done(CHUNKED_PHASE, CHUNKED_KEY, &big())?;
    store.journal.insert(
        chunk_key(CHUNKED_PHASE, CHUNKED_KEY, 0)?,
        b"not an envelope".to_vec(),
    )?;
    let report = store.integrity()?;
    check!(!report.ok);
    let undecodable = findings(&report, CorruptionClass::Undecodable);
    check!(eq; undecodable.len(), 1);
    check!(
        details_contain(&undecodable, "chunk 0 envelope"),
        "the finding must name the chunk and the decode failure: {:?}",
        undecodable.first().map(|finding| finding.detail.as_str())
    );
    check!(findings(&report, CorruptionClass::Truncated).is_empty());
    Ok(())
}

#[test]
fn integrity_reports_an_undecodable_entity_row() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.entities.insert(
        crate::keys::observation_key(Table::Schools, "corrupted", 0),
        b"{not json".to_vec(),
    )?;
    let report = store.integrity()?;
    check!(!report.ok);
    let rows: Vec<&IntegrityCorruption> = report
        .corruptions
        .iter()
        .filter(|finding| finding.subject.starts_with("row/schools/"))
        .collect();
    check!(eq; rows.len(), 1);
    check!(
        eq;
        rows.first().map(|finding| finding.class),
        Some(CorruptionClass::Undecodable)
    );
    Ok(())
}

#[test]
fn integrity_reports_a_decode_limit_instead_of_corruption_for_a_closed_window() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.journal_done(CHUNKED_PHASE, "wi:school1", &json!({"athletes": 1}))?;
    let windowed = store.integrity_with_decode_window(1, 1)?;
    check!(eq; windowed.corruptions, Vec::<IntegrityCorruption>::new());
    check!(
        eq;
        windowed.decode_limits.len(),
        1,
        "a closed window is a limit, not a corruption"
    );
    check!(
        windowed
            .decode_limits
            .iter()
            .all(|limit| limit.contains("stopped at the ADR-036 integrity profile window")),
        "the limit must name the profile window: {:?}",
        windowed.decode_limits
    );
    check!(!windowed.ok);
    let full = store.integrity()?;
    check!(eq; full.decode_limits, Vec::<String>::new());
    check!(full.ok);
    Ok(())
}

#[test]
fn integrity_refuses_to_certify_an_incomplete_decode_scope() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.journal_done(CHUNKED_PHASE, "row", &json!({"athletes": 1}))?;
    let report = store.integrity_with_decode_window(1, 1)?;
    check!(!report.ok, "an undecoded suffix cannot be certified sound");
    check!(eq; report.decode_limits.len(), 1);
    Ok(())
}

#[test]
fn integrity_decodes_unreferenced_physical_journal_chunks() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store
        .journal
        .insert(chunk_key(CHUNKED_PHASE, "orphan", 0)?, b"broken")?;
    let report = store.integrity()?;
    check!(
        !report.ok,
        "an orphan chunk still belongs to the integrity scope"
    );
    check!(eq; findings(&report, CorruptionClass::Undecodable).len(), 1);
    Ok(())
}

#[test]
fn integrity_resolves_ready_aliases_instead_of_only_decoding_the_descriptor() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.journal.insert(
        Store::journal_key("objects", "missing"),
        serde_json::to_vec(&json!({
            "type": "journal-alias", "version": 2, "state": "ready", "object": "a".repeat(64),
        }))?,
    )?;
    let report = store.integrity()?;
    check!(!report.ok);
    check!(eq; findings(&report, CorruptionClass::Truncated).len(), 1);
    Ok(())
}
