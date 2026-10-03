use super::collect::Outcome;
use super::input::Inputs;
use super::{inspect, measure, Evidence, Result};
use serde_json::{json, Value};

pub(super) fn summarize(
    inputs: &Inputs,
    evidence: &Evidence,
    originals_unchanged: bool,
) -> Result<Value> {
    let [_, unbound, _, _, bound, _, _] = &evidence.states;
    let measurements = measurements(inputs, evidence)?;
    let requests = request_count(evidence)?;
    let offline = evidence.binding.offline
        && evidence.attempts.iter().all(|attempt| attempt.offline)
        && requests == 0;
    let executed = evidence
        .attempts
        .iter()
        .all(|attempt| matches!(&attempt.outcome, Outcome::Reported { .. }));
    let retained_originals = retained_originals(evidence);
    let positive_bind = evidence.projections.iter().all(passed)
        && count(bound, "performances")
            .zip(count(unbound, "performances"))
            .is_some_and(|(after, before)| after > before);
    let no_new_source_observations_after_binding = count(bound, "source_observations")
        .zip(count(unbound, "source_observations"))
        .is_some_and(|(after, before)| after == before);
    let success = executed
        && offline
        && originals_unchanged
        && retained_originals
        && positive_bind
        && [
            "unbound_projection",
            "unbound_flush_drop_reopen",
            "bound_flush_drop_reopen",
            "third_replay_idempotence",
        ]
        .iter()
        .all(|key| measurements.get(*key).is_some_and(passed))
        && no_new_source_observations_after_binding
        && evidence.captures.iter().all(passed);
    let Value::Object(mut report) = json!({
        "schema":"qualification_bound_projection_v1", "worker_model":"openai-codex/gpt-6.1-sol",
        "overall_success":success, "success_scope":"generic authentic no-binding -> school binding -> reopened same-key byte-identical public collector replay",
        "evidence_kind":"historical original captures in fresh exclusive offline store",
        "fresh_national_run":false, "national_completion":false, "canonical_identity_accepted":false,
        "lifetime_pr_claimed":false, "source_exhaustion":false, "census_sealed":false,
        "model_calls":0, "actual_requests":requests, "offline":offline,
        "original_source_files_unchanged":originals_unchanged, "owned_cache_originals_unchanged":retained_originals,
        "all_collector_stages_reported":executed, "generic_positive_binding":positive_bind,
        "source_observations_not_duplicated_by_binding":no_new_source_observations_after_binding,
        "physical_digest_schema":"physical_key_value_sha256 uses StoreSnapshot::tables_digest with storage keys/values; decoded_rows_sha256 hashes physical traversal JSONL; content_multiset_sha256 hashes sorted row-digest frequencies",
        "journal_digest_schema":"SHA256 of sorted complete key/payload JSON array per phase; individual journal_payload lookup preserves exact key association; sequence change detects identical-payload rewrites",
        "artifacts":"00..06 stage directories contain seven physical and canonical JSONL tables, complete owned-capture/owned-meet/projection/application journals and state.json; binding.json contains genuine projected roster entities marked not ingested",
        "identity_and_national_acceptance_owner":"Main; no named athlete acceptance assertions",
        "unknown_or_partial_source_behavior":"retained in complete receipts and actual AdapterReport errors/notes; never interpreted as qualification success"
    }) else {
        return Err("qualification summary is not an object".into());
    };
    report.extend(measurements);
    Ok(Value::Object(report))
}

fn retained_originals(evidence: &Evidence) -> bool {
    evidence.states.iter().all(|state| {
        state
            .cache_originals
            .get("unchanged")
            .and_then(Value::as_bool)
            == Some(true)
    })
}

fn measurements(inputs: &Inputs, evidence: &Evidence) -> Result<serde_json::Map<String, Value>> {
    let [before, unbound, reopened_unbound, bound_input, bound, reopened_bound, replayed] =
        &evidence.states;
    let Value::Object(measurements) = json!({
        "input_contract":inputs.facts, "school_binding":evidence.binding, "attempts":evidence.attempts,
        "unbound_projection":inspect::unbound(unbound, inputs),
        "bound_projection":evidence.projections.first(), "replayed_projection":evidence.projections.last(),
        "capture_retention":evidence.captures,
        "unbound_flush_drop_reopen":measure::compare(unbound, reopened_unbound),
        "bound_flush_drop_reopen":measure::compare(bound, reopened_bound),
        "third_replay_idempotence":measure::compare(reopened_bound, replayed),
        "binding_transition_physical_changes":measure::compare(bound_input, bound),
        "before_unbound":before, "after_unbound":unbound, "reopened_unbound":reopened_unbound,
        "before_bound_collect":bound_input, "after_bound":bound,
        "reopened_bound":reopened_bound, "after_replay":replayed
    }) else {
        return Err("qualification measurements are not an object".into());
    };
    Ok(measurements)
}

fn request_count(evidence: &Evidence) -> Result<u64> {
    evidence
        .attempts
        .iter()
        .try_fold(evidence.binding.fetch_stats.requests, |total, attempt| {
            total
                .checked_add(attempt.fetch_stats.requests)
                .ok_or_else(|| "request total overflow".into())
        })
}

fn passed(value: &Value) -> bool {
    value.get("status").and_then(Value::as_str) == Some("PASS")
}

fn count(state: &measure::State, table: &str) -> Option<u64> {
    state.tables.get(table).map(|table| table.physical_rows)
}
