use super::input::{Inputs, API_SHA};
use super::measure::{State, PROJECTION_PHASE};
use super::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use census_crawl::milesplit::{OWNED_CAPTURE_PHASE, OWNED_MEET_PHASE};
use census_store::Store;
use serde_json::{json, Value};

pub(super) fn retained_capture(store: &Store, inputs: &Inputs) -> Result<Value> {
    let mut payloads = store.journal_payloads(OWNED_CAPTURE_PHASE)?;
    payloads.retain(|row| {
        row.get("capture")
            .is_some_and(|capture| capture_matches(capture, inputs))
    });
    let chunk_count = payloads
        .iter()
        .filter(|row| row.get("raw_base64").is_some())
        .count();
    let chunks_match = chunks_match(&payloads, inputs, chunk_count)?;
    let manifest_count = payloads
        .iter()
        .filter(|row| row.get("chunks").is_some())
        .count();
    let manifest_matches = manifest_count == 1
        && payloads.iter().any(|row| {
            row.get("chunks").and_then(Value::as_u64) == u64::try_from(chunk_count).ok()
                && row.get("chunk_bytes").and_then(Value::as_u64) == Some(32 * 1024)
                && row.get("encoding").and_then(Value::as_str) == Some("base64")
                && row
                    .get("capture")
                    .is_some_and(|capture| capture_matches(capture, inputs))
        });
    let parsed_key = format!("parsed/725218/{API_SHA}");
    let interpretation = store.journal_payloads(OWNED_MEET_PHASE)?;
    let parsed_summary = interpretation.iter().any(|row| {
        row.get("disposition").and_then(Value::as_str) == Some("parsed")
            && row.get("published_rows").and_then(Value::as_u64) == Some(602)
            && row.get("owned_rows").and_then(Value::as_u64) == Some(560)
            && row.get("team_relay_rows").and_then(Value::as_u64) == Some(42)
            && row.get("rejected_individual_rows").and_then(Value::as_u64) == Some(0)
            && row
                .get("individual_parse_complete")
                .and_then(Value::as_bool)
                == Some(true)
            && row.get("ownership_complete").and_then(Value::as_bool) == Some(false)
            && row.get("completeness").and_then(Value::as_str) == Some("unknown")
            && row
                .get("capture")
                .is_some_and(|capture| capture_matches(capture, inputs))
    });
    let parsed_receipt = store.journal_keys(OWNED_MEET_PHASE)?.contains(&parsed_key);
    Ok(json!({"capture_chunks_match_original_bytes": chunks_match,
        "capture_manifest_matches_original_metadata": manifest_matches,
        "interpretation_phase": OWNED_MEET_PHASE,
        "original_domain_summary_retained": parsed_summary && parsed_receipt,
        "status": if chunks_match && manifest_matches && parsed_summary && parsed_receipt { "PASS" } else { "FAIL" }}))
}

fn chunks_match(payloads: &[Value], inputs: &Inputs, chunk_count: usize) -> Result<bool> {
    let expected = inputs.api.body.chunks(32 * 1024);
    Ok(chunk_count == expected.len()
        && expected
            .enumerate()
            .try_fold(true, |same, (index, bytes)| -> Result<bool> {
                let index = u64::try_from(index)?;
                let Some(row) = payloads
                    .iter()
                    .find(|row| row.get("chunk_index").and_then(Value::as_u64) == Some(index))
                else {
                    return Ok(false);
                };
                let encoded = row
                    .get("raw_base64")
                    .and_then(Value::as_str)
                    .ok_or("capture chunk has no base64 string")?;
                let decoded = STANDARD.decode(encoded)?;
                Ok(same
                    && decoded == bytes
                    && row
                        .get("capture")
                        .is_some_and(|capture| capture_matches(capture, inputs)))
            })?)
}

fn capture_matches(value: &Value, inputs: &Inputs) -> bool {
    value.get("url").and_then(Value::as_str) == Some(inputs.api.metadata.url.as_str())
        && value.get("method").and_then(Value::as_str) == Some("GET")
        && value.get("status").and_then(Value::as_u64) == Some(200)
        && value.get("content_digest").and_then(Value::as_str)
            == Some(inputs.api.metadata.content_digest.as_str())
        && value.get("bytes").and_then(Value::as_u64) == u64::try_from(inputs.api.body.len()).ok()
        && value.get("fetched_at").and_then(Value::as_str)
            == Some(inputs.api.metadata.fetched_at.as_str())
}

pub(super) fn projection(state: &State, inputs: &Inputs) -> Value {
    let rows = state
        .tables
        .get("source_observations")
        .map(|table| table.physical_rows);
    let expected_rows = inputs
        .facts
        .get("owned_capture")
        .and_then(|capture| capture.get("requested_result_set_individuals"))
        .and_then(Value::as_u64);
    let no_invented_bindings =
        ["athletes", "performances", "teams", "events"]
            .iter()
            .all(|table| {
                state
                    .tables
                    .get(*table)
                    .is_some_and(|table| table.physical_rows == 0)
            });
    let metadata_correct = matches!(
        state
            .meet_metadata
            .get("verification_status")
            .and_then(Value::as_str),
        Some("PASS" | "ABSENT")
    );
    let retained_unresolved = super::measure::unresolved(state);
    let retained_observations = rows.is_some_and(|rows| rows > 0)
        && expected_rows.is_some_and(|expected| rows.is_some_and(|rows| rows >= expected));
    json!({"source_observation_physical_rows": rows, "expected_first_collect_source_rows": expected_rows,
        "retained_owned_observations": retained_observations,
        "no_canonical_binding_invented": no_invented_bindings,
        "partial_v3_receipt_present_complete_receipt_absent": retained_unresolved,
        "metadata_entity_date_sport_correct_if_retained": metadata_correct,
        "phase": PROJECTION_PHASE,
        "status": if no_invented_bindings && metadata_correct && retained_unresolved && retained_observations {
            "PASS"
        } else { "FAIL" }})
}
