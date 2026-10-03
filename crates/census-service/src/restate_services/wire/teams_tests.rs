use std::error::Error;

use serde_json::{json, Value};

use super::{CompletedTeams, JurisdictionState, StageOutcome, TeamsFailure, TeamsStage};

type TestResult = Result<(), Box<dyn Error>>;

fn outcome(errors: Vec<String>) -> StageOutcome {
    StageOutcome {
        records: 37,
        at: "2026-10-01".to_string(),
        errors,
        notes: vec!["wiaa: retained 37 teams".to_string()],
    }
}

#[test]
fn terminal_failure_retains_details_without_completion_or_automatic_work() -> TestResult {
    let state = JurisdictionState {
        identity: "jurisdiction:WI:2026:1".to_string(),
        teams: TeamsStage::Failed(TeamsFailure::ActionTerminal {
            at: "2026-10-01".to_string(),
            code: 429,
            message: "coach directory admission exhausted".to_string(),
        }),
        ..JurisdictionState::default()
    };
    let encoded = serde_json::to_value(&state)?;
    let restored: JurisdictionState = serde_json::from_value(encoded.clone())?;

    check!(eq; restored.identity, state.identity);
    check!(!restored.teams.is_completed());
    check!(!restored.teams.is_owed());
    check!(eq;
        encoded.get("teams"),
        Some(&json!({
            "status": "failed",
            "outcome": {
                "kind": "action_terminal",
                "at": "2026-10-01",
                "code": 429,
                "message": "coach directory admission exhausted"
            }
        }))
    );
    Ok(())
}

#[test]
fn partial_source_outcome_cannot_complete_and_retains_raw_progress() -> TestResult {
    let partial = outcome(vec!["coach_directories: timed out".to_string()]);
    check!(CompletedTeams::try_from(partial.clone()).is_err());
    let stage = TeamsStage::from_outcome(partial.clone(), "2026-10-02".to_string());
    let restored: TeamsStage = serde_json::from_value(serde_json::to_value(&stage)?)?;

    check!(!restored.is_completed());
    check!(!restored.is_owed());
    let TeamsStage::Failed(TeamsFailure::IncompleteOutcome { at, outcome }) = restored else {
        return Err("partial teams outcome lost its incomplete failure variant".into());
    };
    check!(eq; at, "2026-10-02");
    check!(eq;
        serde_json::to_value(outcome)?,
        serde_json::to_value(partial)?
    );
    Ok(())
}

#[test]
fn source_complete_outcome_retains_counts_notes_and_acquisition_date() -> TestResult {
    let complete = outcome(Vec::new());
    let stage = TeamsStage::from_outcome(complete.clone(), "2026-10-02".to_string());
    let restored: TeamsStage = serde_json::from_value(serde_json::to_value(&stage)?)?;

    check!(restored.is_completed());
    check!(!restored.is_owed());
    let TeamsStage::Completed(completed) = restored else {
        return Err("source-complete teams outcome did not remain completed".into());
    };
    check!(eq;
        serde_json::to_value(completed.outcome())?,
        serde_json::to_value(complete)?
    );
    Ok(())
}

#[test]
fn legacy_null_missing_and_untagged_teams_states_fail_closed() -> TestResult {
    let null = json!({"identity": "retained-run", "teams": null});
    let missing = json!({"identity": "retained-run"});
    let untagged = json!({"identity": "retained-run", "teams": outcome(Vec::new())});

    [null, missing, untagged]
        .into_iter()
        .try_for_each(|legacy| {
            let error = serde_json::from_value::<JurisdictionState>(legacy)
                .err()
                .ok_or("legacy teams state was silently accepted")?;
            check!(eq; error.classify(), serde_json::error::Category::Data);
            Ok::<(), Box<dyn Error>>(())
        })
}

#[test]
fn completed_wire_with_source_errors_is_rejected() -> TestResult {
    let invalid = json!({
        "teams": {
            "status": "completed",
            "outcome": outcome(vec!["wiaa: incomplete index".to_string()])
        }
    });
    let error = serde_json::from_value::<JurisdictionState>(invalid)
        .err()
        .ok_or("completed wire state accepted retained source errors")?;

    check!(eq; error.classify(), serde_json::error::Category::Data);
    Ok(())
}

#[test]
fn explicitly_owed_wire_state_remains_eligible_for_initial_teams_work() -> TestResult {
    let state: JurisdictionState = serde_json::from_value(json!({
        "identity": "fresh-run",
        "teams": {"status": "owed"}
    }))?;

    check!(state.teams.is_owed());
    check!(!state.teams.is_completed());
    check!(eq;
        serde_json::to_value(state.teams)?,
        json!({"status": "owed"})
    );
    Ok(())
}

#[test]
fn completed_wrapper_wire_rejects_errors_even_without_stage_envelope() -> TestResult {
    let invalid: Value = serde_json::to_value(outcome(vec!["source refused".to_string()]))?;
    let error = serde_json::from_value::<CompletedTeams>(invalid)
        .err()
        .ok_or("completed wrapper accepted retained source errors")?;

    check!(eq; error.classify(), serde_json::error::Category::Data);
    Ok(())
}
