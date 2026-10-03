use super::binding::Binding;
use super::input::Inputs;
use super::measure::{State, PROJECTION_PHASE};
use super::Result;
use base64::{engine::general_purpose::STANDARD, Engine};
use census_crawl::milesplit::{OwnedPerformance, OWNED_CAPTURE_PHASE};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, Evidence, EvidenceMethod, SourceNamespace,
};
use census_store::{Store, Table};
use serde_json::{json, Value};

pub(super) fn unbound(state: &State, inputs: &Inputs) -> Value {
    let expected = inputs
        .owned
        .rows
        .iter()
        .filter(|row| row.result_set_id == 1266814)
        .count();
    let source_rows = state
        .tables
        .get("source_observations")
        .map(|table| table.physical_rows);
    let no_canonical_population = ["schools", "athletes", "teams", "events", "performances"]
        .iter()
        .all(|name| {
            state
                .tables
                .get(*name)
                .is_some_and(|table| table.physical_rows == 0)
        });
    let partial = state
        .journals
        .get(PROJECTION_PHASE)
        .is_some_and(|phase| phase.keys.iter().any(|key| receipt_key(key, "partial")));
    let complete = state
        .journals
        .get(PROJECTION_PHASE)
        .is_some_and(|phase| phase.keys.iter().any(|key| receipt_key(key, "projection")));
    let retained = source_rows == u64::try_from(expected).ok() && expected > 0;
    json!({"status": if retained && no_canonical_population && partial && !complete { "PASS" } else { "FAIL" },
        "source_physical_rows": source_rows, "expected_source_rows": expected,
        "no_canonical_population_without_binding": no_canonical_population,
        "partial_receipt_present": partial, "complete_receipt_present": complete,
        "missing_school_prerequisite_is_not_hidden": !retained})
}

fn receipt_key(key: &str, disposition: &str) -> bool {
    key.strip_prefix(disposition)
        .and_then(|key| key.strip_prefix("/725218/1266814/"))
        .is_some_and(|digest| {
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
}

pub(super) fn bound(store: &Store, inputs: &Inputs, binding: &Binding) -> Result<Value> {
    let schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
    let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    let teams: Vec<CanonicalTeam> = store.scan(Table::Teams)?;
    let events: Vec<CanonicalEvent> = store.scan(Table::Events)?;
    let meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
    let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    let parents_join = athletes
        .iter()
        .all(|athlete| schools.iter().any(|school| school.id == athlete.school))
        && teams
            .iter()
            .all(|team| schools.iter().any(|school| school.id == team.school));
    let ownership: Vec<_> = performances
        .iter()
        .map(|performance| {
            performance_evidence(
                performance,
                (&athletes, &teams, &events, &meets),
                inputs,
                binding,
            )
        })
        .collect::<Result<_>>()?;
    let exact_rows = ownership
        .iter()
        .all(|row| row.get("status").and_then(Value::as_str) == Some("PASS"));
    let positive = !ownership.is_empty() && exact_rows && parents_join;
    let retained = store.journal_payloads(PROJECTION_PHASE)?;
    let projected = retained
        .iter()
        .filter(|row| row.get("disposition").and_then(Value::as_str) == Some("projected"))
        .count();
    let unresolved = retained
        .iter()
        .filter(|row| row.get("disposition").and_then(Value::as_str) == Some("retained_unresolved"))
        .count();
    let bound_source_rows = inputs
        .owned
        .rows
        .iter()
        .filter(|row| row.result_set_id == 1266814 && row.team_id == binding.provider_team_id)
        .count();
    Ok(
        json!({"status": if positive { "PASS" } else { "FAIL" }, "qualified_provider_team_id": binding.provider_team_id,
        "qualified_school_id": binding.school.id, "school_rows_retained": schools.len(),
        "athlete_parents_and_team_schools_join": parents_join, "canonical_performance_rows": ownership.len(),
        "exact_source_owned_performance_rows": ownership, "bound_source_individual_rows": bound_source_rows,
        "projected_row_receipts": projected, "retained_unresolved_row_receipts": unresolved,
        "canonical_identity_acceptance": "not evaluated", "source_completeness": inputs.owned.completeness}),
    )
}

type Parents<'a> = (
    &'a [CanonicalAthlete],
    &'a [CanonicalTeam],
    &'a [CanonicalEvent],
    &'a [CanonicalMeet],
);

fn performance_evidence(
    performance: &CanonicalPerformance,
    parents: Parents<'_>,
    inputs: &Inputs,
    binding: &Binding,
) -> Result<Value> {
    let (athletes, teams, events, meets) = parents;
    let [api, raw, _, _] = &inputs.captures;
    let notes = performance
        .evidence
        .iter()
        .filter_map(|evidence| evidence.note.as_deref())
        .map(serde_json::from_str::<Value>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let row = notes
        .iter()
        .filter_map(|note| note.get("result_id").and_then(Value::as_u64))
        .find_map(|id| inputs.owned.rows.iter().find(|row| row.result_id == id));
    let athlete = athletes
        .iter()
        .find(|athlete| athlete.id == performance.athlete);
    let team = teams.iter().find(|team| team.id == performance.team);
    let event = events.iter().find(|event| event.id == performance.event);
    let meet = meets.iter().find(|meet| meet.id == performance.meet);
    let source_exact = row.is_some_and(|row| {
        row.team_id == binding.provider_team_id
            && row.result_set_id == 1266814
            && performance.source_athlete.as_ref() == Some(&row.source_athlete)
            && performance.mark == row.mark
            && performance.timing == row.timing
            && notes.iter().any(|note| note_matches(note, row, api))
            && athlete.is_some_and(|athlete| {
                athlete.school == binding.school.id
                    && athlete.identity_in(&SourceNamespace::MilesplitAthlete)
                        == Some(&row.source_athlete)
            })
            && team.is_some_and(|team| {
                team.school == binding.school.id && team.school_year.get() == 2025
            })
            && event
                .is_some_and(|event| event.kind == row.event_kind && event.meet == performance.meet)
    });
    let acquisition_exact = performance
        .evidence
        .iter()
        .any(|evidence| original_evidence(evidence, &api.metadata.url, &api.metadata.fetched_at));
    let metadata_exact = meet
        .is_some_and(|meet| meet.date == "2026-03-27" && performance.date == meet.date)
        && performance.evidence.iter().any(|evidence| {
            original_evidence(evidence, &raw.metadata.url, &raw.metadata.fetched_at)
        });
    Ok(
        json!({"status": if source_exact && acquisition_exact && metadata_exact { "PASS" } else { "FAIL" },
        "performance_id": performance.id, "source_key": performance.source_key, "source_row": row,
        "source_ownership_event_mark_exact": source_exact, "original_acquisition_evidence": acquisition_exact,
        "original_meet_metadata": metadata_exact, "performance": performance}),
    )
}

fn note_matches(note: &Value, row: &OwnedPerformance, api: &super::input::Capture) -> bool {
    note.get("result_id").and_then(Value::as_u64) == Some(row.result_id)
        && note.get("team_id").and_then(Value::as_u64) == Some(row.team_id)
        && note.get("locator").and_then(Value::as_str) == Some(row.locator.as_str())
        && note.get("sha256").and_then(Value::as_str) == Some(api.metadata.content_digest.as_str())
        && note.get("capture_url").and_then(Value::as_str) == Some(api.metadata.url.as_str())
        && note.get("acquired_at").and_then(Value::as_str) == Some(api.metadata.fetched_at.as_str())
        && note.get("provider") == Some(&row.provider)
}

fn original_evidence(evidence: &Evidence, url: &str, acquired_at: &str) -> bool {
    evidence.method == EvidenceMethod::Parsed
        && evidence.source.url.as_deref() == Some(url)
        && evidence.observed_on == acquired_at
}

pub(super) fn retained_capture(store: &Store, inputs: &Inputs) -> Result<Value> {
    let [api, _, _, _] = &inputs.captures;
    let mut payloads = store.journal_payloads(OWNED_CAPTURE_PHASE)?;
    payloads.retain(|row| {
        row.get("capture")
            .is_some_and(|capture| capture_matches(capture, api))
    });
    let chunks = payloads
        .iter()
        .filter(|row| row.get("raw_base64").is_some())
        .count();
    let expected = api.body.chunks(32 * 1024);
    let identical = expected.len() == chunks
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
                    .ok_or("capture chunk encoding absent")?;
                let capture = row
                    .get("capture")
                    .ok_or("capture chunk provenance absent")?;
                Ok(same && STANDARD.decode(encoded)? == bytes && capture_matches(capture, api))
            })?;
    let manifest = payloads.iter().any(|row| {
        row.get("chunks").and_then(Value::as_u64) == u64::try_from(chunks).ok()
            && row
                .get("capture")
                .is_some_and(|capture| capture_matches(capture, api))
    });
    Ok(
        json!({"status": if identical && manifest { "PASS" } else { "FAIL" },
        "chunks": chunks, "original_api_bytes_retained": identical, "original_metadata_manifest": manifest}),
    )
}

fn capture_matches(value: &Value, capture: &super::input::Capture) -> bool {
    value.get("url").and_then(Value::as_str) == Some(capture.metadata.url.as_str())
        && value.get("method").and_then(Value::as_str) == Some("GET")
        && value.get("status").and_then(Value::as_u64) == Some(200)
        && value.get("content_digest").and_then(Value::as_str)
            == Some(capture.metadata.content_digest.as_str())
        && value.get("bytes").and_then(Value::as_u64) == u64::try_from(capture.body.len()).ok()
        && value.get("fetched_at").and_then(Value::as_str)
            == Some(capture.metadata.fetched_at.as_str())
}
