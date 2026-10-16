use super::{Expected, TestResult};
use census_domain::model::{serialized_digest, Evidence};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

const RAW_URL: &str = "https://al.milesplit.com/meets/725218/results/1266814/raw";
const ACQUIRED_AT: &str = "2026-10-01T23:44:16Z";

pub(super) fn retained(directory: &Path, expected: &Expected) -> TestResult {
    let frozen: Value =
        serde_json::from_slice(&std::fs::read(directory.join("frozen-input.json"))?)?;
    let performances: Vec<census_domain::model::CanonicalPerformance> = serde_json::from_value(
        frozen
            .pointer("/data/performances")
            .ok_or("frozen performances")?
            .clone(),
    )?;
    for expected in [&expected.unresolved, &expected.excluded] {
        let retained = performances
            .iter()
            .find(|row| row.id == expected.id)
            .ok_or("retained raw performance")?;
        check!(eq; retained, expected);
    }
    let refs = expected
        .unresolved
        .evidence
        .iter()
        .map(|fact| serialized_digest(&("performance", expected.unresolved.id.as_str(), fact)))
        .collect::<Result<BTreeSet<_>, _>>()?;
    check!(eq; refs.len(), 2);
    let raw_refs = expected
        .unresolved
        .evidence
        .iter()
        .filter(|fact| fact.source.url.as_deref() == Some(RAW_URL))
        .count();
    check!(eq; raw_refs, 1);
    for fact in &expected.unresolved.evidence {
        check!(eq; fact.source.id, "milesplit_al");
        check!(eq; fact.observed_on, ACQUIRED_AT);
        source_note(fact)?;
    }
    Ok(())
}

fn source_note(fact: &Evidence) -> TestResult {
    let note: Value = serde_json::from_str(fact.note.as_deref().ok_or("source evidence note")?)?;
    let owned_url = format!(
        "https://al.milesplit.com/api/v1/meets/725218/performances?isMeetPro=0&fields={}",
        census_crawl::milesplit::OWNED_FIELDS
    );
    check!(eq; fact.source.id.as_str(), "milesplit_al");
    check!(eq; fact.observed_on.as_str(), ACQUIRED_AT);
    match fact.source.url.as_deref() {
        Some(RAW_URL) => raw_note(&note, &owned_url),
        Some(url) if url == owned_url => {
            check!(eq; note.get("result_id").and_then(Value::as_u64), Some(201782263));
            for (pointer, expected) in [
                ("/capture_url", owned_url.as_str()),
                ("/raw_metadata_url", RAW_URL),
                ("/acquired_at", ACQUIRED_AT),
                ("/meet_name", "Troy Invitational #2"),
                ("/meet_date", "2026-03-27"),
                ("/provider/divisionName", "High School"),
                ("/provider/mark", "13-9"),
                ("/provider/meetId", "725218"),
                ("/provider/meetResultsId", "1266814"),
            ] {
                check!(eq; note.pointer(pointer).and_then(Value::as_str), Some(expected));
            }
            Ok(())
        }
        _ => Err("unexpected retained evidence source".into()),
    }
}

fn raw_note(note: &Value, owned_url: &str) -> TestResult {
    for (pointer, expected) in [
        ("/role", "raw result file meet/season metadata only"),
        ("/meet_id", "725218"),
        ("/result_set_id", "1266814"),
        ("/meet_name", "Troy Invitational #2"),
        ("/meet_date", "2026-03-27"),
        ("/owned_capture/url", owned_url),
        ("/owned_capture/fetched_at", ACQUIRED_AT),
        ("/raw_metadata_capture/url", RAW_URL),
        ("/raw_metadata_capture/fetched_at", ACQUIRED_AT),
        ("/parser_revision", "milesplit_owned_v4_raw_metadata_v1"),
    ] {
        check!(eq; note.pointer(pointer).and_then(Value::as_str), Some(expected));
    }
    Ok(())
}
