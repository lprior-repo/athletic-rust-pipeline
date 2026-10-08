use anyhow::{Context, Result};
use census_service::restate_services::{JurisdictionState, TeamsFailure, TeamsStage};
use serde_json::{json, Value};

use super::super::artifacts::{oracle, rows};
use super::super::scenario::Run;
pub(super) fn tls_failure(first: &Run) -> bool {
    !first.physical.is_empty()
        && first.physical.iter().all(|request| {
            request.get("response_status") == Some(&Value::Null)
                && request.get("forwarded").and_then(Value::as_bool) == Some(false)
        })
        && first.physical.iter().any(|request| {
            request
                .get("tls_handshake_record_observed")
                .and_then(Value::as_bool)
                == Some(true)
        })
}

pub(super) fn independent_checks(state: &JurisdictionState, persisted: bool) -> [Value; 4] {
    let stages = json!({"rosters":state.rosters, "history":state.history,
        "history_window":state.history_window});
    let later = state.rosters.is_some() && meets_persisted(state) && results_persisted(state);
    [
        oracle(
            "independent_rosters_persisted",
            state.rosters.is_some() && persisted,
            json!({"rosters":state.rosters, "national_completeness":false}),
        ),
        oracle(
            "independent_meets_persisted",
            meets_persisted(state) && persisted,
            json!({"meets":state.history.meets, "national_completeness":false}),
        ),
        oracle(
            "independent_results_persisted",
            results_persisted(state) && persisted,
            json!({"results":state.history.results, "national_completeness":false}),
        ),
        oracle(
            "independent_stages_ran_and_persisted",
            later && persisted,
            stages,
        ),
    ]
}

pub(super) fn repeat_checks(
    first: &Run,
    repeat: &Run,
    state: &JurisdictionState,
    repeated: &JurisdictionState,
) -> Result<[Value; 2]> {
    let teams_unchanged = first.state.get("teams") == repeat.state.get("teams")
        && matches!(repeated.teams, TeamsStage::Failed(_));
    let repeat_wire = repeat
        .physical
        .len()
        .checked_sub(first.physical.len())
        .context("physical evidence shrank during parent repeat")?;
    let refusal = refuses_teams(first, state) && refuses_teams(repeat, repeated);
    let no_child = rows(&repeat.source_invocations)?.is_empty();
    let prefix = repeat.physical.get(..first.physical.len()) == Some(first.physical.as_slice());
    Ok([
        oracle(
            "final_report_refuses_teams_completion",
            refusal,
            json!({"first_output":first.output, "repeat_output":repeat.output}),
        ),
        oracle(
            "same_parent_key_no_fresh_teams_child_or_physical_request",
            first.id != repeat.id
                && teams_unchanged
                && refusal
                && no_child
                && prefix
                && repeat_wire == 0,
            json!({"first_id":first.id, "repeat_id":repeat.id, "logical_identity":first.state.get("identity"),
                "teams_unchanged":teams_unchanged, "repeat_physical_admissions":repeat_wire,
                "repeat_child_invocations":repeat.source_invocations,
                "limit":"any independent-stage repeat acquisition leaves global zero-admission obligation unproven; source-only replay is measured separately"}),
        ),
    ])
}

fn refuses_teams(run: &Run, state: &JurisdictionState) -> bool {
    let TeamsStage::Failed(TeamsFailure::SourceFailures { outcome, .. }) = &state.teams else {
        return false;
    };
    !(200..300).contains(&run.output.status)
        && run.output.error_source.as_deref() == Some("invocation")
        && run
            .output
            .body
            .get("message")
            .and_then(Value::as_str)
            .is_some_and(|text| {
                text.contains("teams stage remains incomplete")
                    && !outcome.errors.is_empty()
                    && outcome.errors.iter().all(|error| text.contains(error))
            })
}

pub(super) fn reached_boundary(state: &JurisdictionState) -> &'static str {
    if !matches!(
        state.teams,
        TeamsStage::Failed(TeamsFailure::SourceFailures { .. })
    ) {
        return "typed teams SourceFailures state not reached";
    }
    if state.rosters.is_none() {
        return "teams SourceFailures persisted; production rosters did not persist";
    }
    if !meets_persisted(state) {
        return "teams SourceFailures and rosters persisted; production meets did not persist; results not reached";
    }
    if !results_persisted(state) {
        return "teams SourceFailures, rosters and meets persisted; production results did not persist";
    }
    "teams SourceFailures and all independent stages persisted; final refusal and source proof obligations observed separately"
}

fn meets_persisted(state: &JurisdictionState) -> bool {
    state.history_window.as_ref().is_some_and(|window| {
        window
            .years()
            .all(|year| state.history.meets.contains_key(&year))
    })
}

fn results_persisted(state: &JurisdictionState) -> bool {
    state.history_window.as_ref().is_some_and(|window| {
        window
            .years()
            .all(|year| state.history.results.contains_key(&year))
    })
}

pub(super) fn admin_matches(run: &Run) -> Result<bool> {
    let stored = rows(&run.admin_state)?
        .iter()
        .find(|row| row.get("key").and_then(Value::as_str) == Some("state"));
    let Some(text) = stored
        .and_then(|row| row.get("value_utf8"))
        .and_then(Value::as_str)
    else {
        return Ok(false);
    };
    let decoded: Value = serde_json::from_str(text)?;
    Ok(decoded == run.state)
}
