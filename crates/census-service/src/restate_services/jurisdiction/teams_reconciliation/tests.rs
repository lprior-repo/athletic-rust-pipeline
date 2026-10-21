use super::*;
use crate::restate_services::wire::{JurisdictionState, SourcePlan, TeamsSourceFailure};
use census_domain::model::serialized_digest as digest;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn seed() -> Result<(JurisdictionState, TeamsReconciliationRequest), Box<dyn std::error::Error>> {
    let jurisdiction =
        crate::restate_services::jurisdiction::team_source::tests::request()?.jurisdiction;
    let at = "2026-10-02";
    let original = TeamsSourceOutcome::Terminal {
        message: "origin held".into(),
        progress: Vec::new(),
    };
    let state = JurisdictionState {
        history_window: Some(jurisdiction.history),
        plan: Some(SourcePlan {
            sweepable: vec!["milesplit".into()],
            fingerprint: "bound-source-plan".into(),
            ..SourcePlan::default()
        }),
        teams: TeamsStage::Failed(TeamsFailure::SourceFailures {
            at: at.into(),
            outcome: team_collection::empty_outcome(at, CollectionDisposition::Partial),
            failures: vec![TeamsSourceFailure {
                source: "milesplit".into(),
                outcome: original,
            }],
        }),
        ..JurisdictionState::default()
    };
    let request = TeamsReconciliationRequest {
        jurisdiction,
        expected_teams_digest: digest(&state.teams)?,
        expected_plan_digest: digest(&state.plan)?,
    };
    Ok((state, request))
}

#[test]
fn unchanged_parent_returns_the_exact_original_source_outcome() -> TestResult {
    let (state, request) = seed()?;
    let original =
        require_parent(&state, &request).map_err(crate::restate_services::tests::sdk_error)?;
    check!(
        matches!(original, TeamsSourceOutcome::Terminal { ref message, .. } if message == "origin held")
    );
    Ok(())
}

#[test]
fn changed_aggregate_or_source_plan_is_refused_without_mutating_parent() -> TestResult {
    let (state, mut request) = seed()?;
    let before = serde_json::to_vec(&state)?;
    request.expected_teams_digest = "foreign-stage".into();
    check!(require_parent(&state, &request).is_err());
    request.expected_teams_digest = digest(&state.teams)?;
    request.expected_plan_digest = "foreign-plan".into();
    check!(require_parent(&state, &request).is_err());
    check!(eq; serde_json::to_vec(&state)?, before);
    Ok(())
}

#[test]
fn changed_history_scope_is_refused_after_both_parent_digests_match() -> TestResult {
    let (mut state, request) = seed()?;
    state.history_window = None;
    check!(require_parent(&state, &request).is_err());
    Ok(())
}

#[test]
fn already_materialized_downstream_work_is_refused() -> TestResult {
    let (mut state, request) = seed()?;
    state.consolidated = Some(Vec::new());
    check!(require_parent(&state, &request).is_err());
    state.consolidated = None;
    state.rosters = Some(crate::census::StateProgress {
        jurisdiction: request.jurisdiction.jurisdiction,
        rosters_total: 0,
        rosters_committed: 0,
        rosters_remaining: 0,
        rosters_skipped: 0,
        athletes: 0,
        class_of_2027: 0,
        class_of_2027_boys: 0,
        class_of_2027_girls: 0,
        errors: Vec::new(),
        blocked: false,
        blocked_skipped: 0,
        teams: 0,
    });
    check!(require_parent(&state, &request).is_err());
    Ok(())
}

#[test]
fn missing_or_duplicate_original_failure_is_refused_even_with_updated_digest() -> TestResult {
    let (mut state, mut request) = seed()?;
    let TeamsStage::Failed(TeamsFailure::SourceFailures { failures, .. }) = &mut state.teams else {
        return Err("seed source failure missing".into());
    };
    let original = failures.first().ok_or("seed failure absent")?.clone();
    failures.clear();
    request.expected_teams_digest = digest(&state.teams)?;
    check!(require_parent(&state, &request).is_err());
    let TeamsStage::Failed(TeamsFailure::SourceFailures { failures, .. }) = &mut state.teams else {
        return Err("seed source failure changed type".into());
    };
    failures.extend([original.clone(), original]);
    request.expected_teams_digest = digest(&state.teams)?;
    check!(require_parent(&state, &request).is_err());
    Ok(())
}

#[test]
fn missing_or_duplicate_planned_source_is_refused_with_matching_plan_digest() -> TestResult {
    let (mut state, mut request) = seed()?;
    let plan = state.plan.as_mut().ok_or("seed source plan absent")?;
    plan.sweepable.push("milesplit".into());
    request.expected_plan_digest = digest(&state.plan)?;
    check!(require_parent(&state, &request).is_err());
    let plan = state.plan.as_mut().ok_or("seed source plan absent")?;
    plan.sweepable.clear();
    request.expected_plan_digest = digest(&state.plan)?;
    check!(require_parent(&state, &request).is_err());
    Ok(())
}

#[test]
fn unplanned_failure_is_not_silently_dropped_from_reconciliation() -> TestResult {
    let (mut state, mut request) = seed()?;
    let TeamsStage::Failed(TeamsFailure::SourceFailures { failures, .. }) = &mut state.teams else {
        return Err("seed source failure missing".into());
    };
    failures.push(TeamsSourceFailure {
        source: "unplanned-source".into(),
        outcome: TeamsSourceOutcome::Terminal {
            message: "retained unrelated failure".into(),
            progress: Vec::new(),
        },
    });
    request.expected_teams_digest = digest(&state.teams)?;
    check!(require_parent(&state, &request).is_err());
    Ok(())
}
