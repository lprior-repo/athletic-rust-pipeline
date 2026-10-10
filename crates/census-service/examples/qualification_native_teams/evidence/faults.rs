use anyhow::Result;
use census_service::restate_services::TeamsSourceOutcome;
use serde_json::{json, Value};

use super::super::artifacts::{oracle, rows};
use super::super::ledger::Snapshot;
use super::super::scenario::{Faults, Interrupted, Permanent};
use super::action;

mod slots;
#[cfg(test)]
mod tests;
mod timeline;

pub(super) fn checks(faults: &Faults, cold: Option<&Snapshot>) -> Result<Vec<Value>> {
    let permanent = match (&faults.permanent, cold) {
        (Some(witness), Some(cold)) => permanent(witness, cold)?,
        _ => vec![oracle(
            "permanent_source_first_attempt_refusal",
            false,
            json!({"reason":"native in-acquisition fixed-budget policy witness or cold evidence absent", "errors":faults.errors}),
        )],
    };
    let interrupted = match (&faults.interruption, cold) {
        (Some(witness), Some(cold)) => interruption(witness, cold)?,
        _ => vec![oracle(
            "native_interruption_witnessed_final_reservation_boundary",
            false,
            json!({"reason":"actual Unknown3 ingress inspection, held transport, orderly reap, cold slots or restart evidence absent", "errors":faults.errors}),
        )],
    };
    Ok(permanent.into_iter().chain(interrupted).chain([
        oracle("permanent_fetch_network_response_refusal", false,
            json!({"status":"UNPROVEN", "reason":"fixed serving-owner budget refusal is not an HTTP/auth/robots/publisher response"})),
        oracle("full_machine_reset_and_real_clock_fault_qualification", false,
            json!({"status":"UNPROVEN", "reason":"endpoint TERM/restart with native owner retained is not machine-reset or clock-fault evidence"})),
        oracle("fresh_positive_source_acquisition_qualification", false,
            json!({"status":"UNPROVEN", "reason":"owned TLS EOF/hold and policy refusal do not prove source-positive acquisition"})),
    ]).collect())
}

fn permanent(witness: &Permanent, cold: &Snapshot) -> Result<Vec<Value>> {
    let history = action::history(
        &witness.source,
        cold,
        &witness.input.jurisdiction,
        &witness.input.observed_on,
    )?;
    let classified = matches!(&witness.source.outcome,
        Some(TeamsSourceOutcome::Terminal { message, .. })
            if message.contains("source parallelism mismatch")
                && message.contains("fixed at 1") && message.contains("requires 2"));
    let replay = action::checks(
        &witness.source,
        &history,
        std::slice::from_ref(&witness.replay),
    )?;
    let native = direct(
        &witness.source.invocation,
        &witness.source.key,
        &witness.source.id,
    );
    let valid = classified
        && witness.input.jurisdiction.source_parallelism == 2
        && history.get("consistent") == Some(&json!(true))
        && history.get("reservations") == Some(&json!([1]))
        && history.get("recorded_outcomes") == Some(&json!([1]))
        && native
        && witness.before == witness.after
        && replay.iter().all(passed);
    Ok(std::iter::once(oracle("permanent_source_first_attempt_refusal", valid,
        json!({"cause":"real fixed serving-owner source-parallelism policy refusal inside acquisition",
            "native_direct_identity":native, "classified_fixed_budget_refusal":classified,
            "physical_admissions_added":witness.after.len().checked_sub(witness.before.len()),
            "publisher_or_network_response":false, "history":history, "witness":witness})))
        .chain(replay).collect())
}

fn interruption(witness: &Interrupted, cold: &Snapshot) -> Result<Vec<Value>> {
    let history = action::history(
        &witness.settlement,
        cold,
        &witness.input.jurisdiction,
        &witness.input.observed_on,
    )?;
    let boundary = slots::boundary(
        &witness.cold_before,
        &witness.key,
        &witness.input,
        &witness.progress.body,
    )?;
    let replay = action::checks(
        &witness.settlement,
        &history,
        std::slice::from_ref(&witness.replay),
    )?;
    let native = direct(
        &witness.native_at_witness,
        &witness.key,
        &witness.original_id,
    ) && witness
        .native_at_witness
        .get("status")
        .and_then(Value::as_str)
        .is_some_and(|status| !matches!(status, "completed" | "killed" | "paused"))
        && rows(&witness.original_terminal)?.iter().any(|row| {
            row.get("id").and_then(Value::as_str) == Some(witness.original_id.as_str())
                && row
                    .get("status")
                    .and_then(Value::as_str)
                    .is_some_and(|status| matches!(status, "completed" | "killed"))
        })
        && direct(
            &witness.settlement.invocation,
            &witness.key,
            &witness.settlement.id,
        );
    let retained = matches!(
        witness.settlement.outcome,
        Some(TeamsSourceOutcome::Interrupted {
            attempts: Some(3),
            ..
        })
    ) && witness.settlement.key == witness.key
        && witness.settlement.id != witness.original_id
        && history.get("consistent") == Some(&json!(true))
        && history.get("recorded_outcomes") == Some(&json!([1, 2]))
        && slots::unchanged(&witness.cold_before, cold, &witness.key);
    let inspection = witness.progress.method == "POST"
        && (200..300).contains(&witness.progress.status)
        && progress_route(&witness.progress.url, &witness.key);
    let ordered = timeline::matches(witness);
    let physical_unchanged = witness.physical_at_witness == witness.physical_after;
    let valid = boundary
        && native
        && retained
        && inspection
        && ordered
        && physical_unchanged
        && replay.iter().all(passed);
    Ok(std::iter::once(oracle("native_interruption_witnessed_final_reservation_boundary", valid,
        json!({"boundary_slots_match_actual_progress":boundary, "native_identity":native,
            "retained_interrupted_some_three":retained, "actual_ingress_progress":inspection,
            "held_inspection_term_reap_release_restart_ordered":ordered,
            "physical_admissions_unchanged_after_witness":physical_unchanged,
            "restart_separated_slot_payload_identity":slots::unchanged(&witness.cold_before, cold, &witness.key),
            "history":history, "witness":witness, "machine_reset":false})))
        .chain(replay).collect())
}

fn direct(row: &Value, key: &str, id: &str) -> bool {
    row.get("id").and_then(Value::as_str) == Some(id)
        && row.get("target_service_name").and_then(Value::as_str) == Some("TeamsSource")
        && row.get("target_service_key").and_then(Value::as_str) == Some(key)
        && row.get("target_handler_name").and_then(Value::as_str) == Some("run")
        && row.get("invoked_by").and_then(Value::as_str) == Some("ingress")
        && row.get("caller_service_absent").and_then(Value::as_bool) == Some(true)
        && row.get("caller_id_absent").and_then(Value::as_bool) == Some(true)
        && row
            .get("invoked_by_service_name")
            .is_none_or(Value::is_null)
        && row.get("invoked_by_id").is_none_or(Value::is_null)
}

fn progress_route(raw: &str, key: &str) -> bool {
    let Ok(url) = url::Url::parse(raw) else {
        return false;
    };
    if url.host_str() != Some("127.0.0.1")
        || url.scheme() != "http"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return false;
    }
    let mut expected = url.clone();
    let Ok(mut path) = expected.path_segments_mut() else {
        return false;
    };
    path.clear().extend(["TeamsSource", key, "progress"]);
    drop(path);
    url == expected
}

fn passed(check: &Value) -> bool {
    check.get("status").and_then(Value::as_str) == Some("PASS")
}
