use census_domain::model::Revision;
use census_domain::model::{RunManifest, SchoolYear};
use restate_sdk::prelude::{HandlerError, TerminalError};

use crate::census::{MeetCensus, StateProgress};
use crate::restate_services::jobs;
use crate::restate_services::results_arms::ResultsStageOutcome;
use crate::restate_services::wire::{
    HistoryWindow, JurisdictionRequest, JurisdictionState, TeamsFailure, TeamsStage,
};

#[derive(Clone, Copy)]
pub(in crate::restate_services) enum Stage {
    Teams,
    Rosters,
    Meets,
    Results,
}

impl Stage {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Teams => "teams",
            Self::Rosters => "rosters",
            Self::Meets => "meets",
            Self::Results => "results",
        }
    }
}

pub(in crate::restate_services) fn roster_stage_owed(state: &JurisdictionState) -> bool {
    match state.rosters.as_ref() {
        None => true,
        Some(progress) => !progress.is_terminal(),
    }
}

pub(in crate::restate_services) fn coach_lane_owed(state: &JurisdictionState) -> bool {
    match state.coach_lane.as_ref() {
        None => true,
        Some(lane) => !lane.is_terminal(),
    }
}

pub(in crate::restate_services) fn bound_run_refusal(
    manifest: Option<&RunManifest>,
    season: SchoolYear,
    revision: Revision,
) -> Option<String> {
    let manifest = manifest?;
    let bound = manifest.run;
    (bound.season() != season || bound.revision() != revision.get()).then(|| {
        format!(
            "this store carries census run {}-{} ({} jurisdictions, cohort {}) and refuses jurisdiction work for {}-{}: submit the bound revision or start a fresh store root",
            bound.season().get(),
            bound.revision(),
            manifest.jurisdictions.len(),
            manifest.cohort,
            season.get(),
            revision.get()
        )
    })
}

pub(in crate::restate_services) fn stage_owed(
    stage: Stage,
    state: &JurisdictionState,
    request: &JurisdictionRequest,
) -> bool {
    match stage {
        Stage::Teams => state.teams.is_resumable(),
        Stage::Rosters => roster_stage_owed(state),
        Stage::Meets => !state.history.meets_terminal(&request.history),
        Stage::Results => !state.history.results_terminal(&request.history),
    }
}

pub(in crate::restate_services) fn owed_work(
    state: &JurisdictionState,
    request: &JurisdictionRequest,
) -> (Vec<String>, Vec<String>) {
    let mut owed: Vec<String> = Vec::new();
    let mut reasons: Vec<String> = Vec::new();
    for stage in [Stage::Teams, Stage::Rosters, Stage::Meets, Stage::Results] {
        if !stage_owed(stage, state, request) {
            continue;
        }
        owed.push(stage.name().to_string());
        reasons.push(match stage {
            Stage::Teams => teams_reason(&state.teams),
            Stage::Rosters => rosters_reason(state.rosters.as_ref()),
            Stage::Meets => meets_reason(state, request),
            Stage::Results => results_reason(state, request),
        });
    }
    if coach_lane_owed(state) {
        owed.push("coach_lane".to_string());
        reasons.push(coach_lane_reason(state));
    }
    match state.athleticlive.as_ref() {
        Some(lane) if !lane.is_terminal() => {
            owed.push("athleticlive_manifest".to_string());
            reasons.push(lane.reason());
        }
        None if super::athleticlive::ledger::applicable(request) => {
            owed.push("athleticlive_manifest".to_string());
            reasons.push(super::athleticlive::ledger::missing_reason(request));
        }
        Some(_) | None => {}
    }
    if let Some(plan) = state.plan.as_ref() {
        for refused in &plan.refused {
            reasons.push(format!("refused {}: {}", refused.slug, refused.reason));
        }
    }
    (owed, reasons)
}

fn teams_reason(stage: &TeamsStage) -> String {
    match stage {
        TeamsStage::Owed => "teams: no attempt recorded".to_string(),
        TeamsStage::Completed(completed) => {
            format!("teams: retained {} records", completed.outcome().records)
        }
        TeamsStage::Failed(TeamsFailure::ActionTerminal { at, code, message }) => {
            format!("teams: terminal action at {at} code {code}: {message}")
        }
        TeamsStage::Failed(TeamsFailure::IncompleteOutcome { at, outcome })
        | TeamsStage::Failed(TeamsFailure::SourceFailures { at, outcome, .. }) => format!(
            "teams: incomplete at {at}: records={} errors={}",
            outcome.records,
            outcome.errors.len()
        ),
    }
}

fn rosters_reason(progress: Option<&StateProgress>) -> String {
    match progress {
        None => "rosters: no outcome recorded".to_string(),
        Some(progress) => format!(
            "rosters: committed={} remaining={} skipped={} errors={} blocked={}",
            progress.rosters_committed,
            progress.rosters_remaining,
            progress.rosters_skipped,
            progress.errors.len(),
            progress.blocked
        ),
    }
}

fn meets_reason(state: &JurisdictionState, request: &JurisdictionRequest) -> String {
    let mut parts: Vec<String> = Vec::new();
    for year in request.history.years() {
        if state
            .history
            .meets
            .get(&year)
            .is_some_and(MeetCensus::is_terminal)
        {
            continue;
        }
        match state.history.meets.get(&year) {
            Some(meet) => parts.push(format!(
                "{year}:pages={} rows={} truncated={} repeated={}",
                meet.pages, meet.rows, meet.truncated, meet.repeated
            )),
            None => parts.push(format!("{year}:absent")),
        }
    }
    format!("meets: {}", parts.join(" "))
}

fn results_reason(state: &JurisdictionState, request: &JurisdictionRequest) -> String {
    let mut parts: Vec<String> = Vec::new();
    for year in request.history.years() {
        if state
            .history
            .results
            .get(&year)
            .is_some_and(ResultsStageOutcome::is_terminal)
        {
            continue;
        }
        match state.history.results.get(&year) {
            Some(outcome) => parts.push(format!(
                "{year}:sources={} pending={}",
                outcome.per_source.len(),
                outcome.pending.len()
            )),
            None => parts.push(format!("{year}:absent")),
        }
    }
    format!("results: {}", parts.join(" "))
}

fn coach_lane_reason(state: &JurisdictionState) -> String {
    match state.coach_lane.as_ref() {
        None => "coach_lane: no obligation recorded".to_string(),
        Some(lane) => format!(
            "coach_lane: {} under arm {} (updated {})",
            lane.status().slug(),
            lane.arm_name(),
            lane.updated_on()
        ),
    }
}

pub(in crate::restate_services) fn bind_history_window(
    state: &mut JurisdictionState,
    window: HistoryWindow,
) -> Result<(), HandlerError> {
    if state.history_window.is_some_and(|bound| bound != window) {
        return Err(jobs::invariant(
            "history scope changed within an existing logical run",
        ));
    }
    state.history_window = Some(window);
    Ok(())
}

pub(in crate::restate_services) fn teams_failure(failure: &TeamsFailure) -> HandlerError {
    match failure {
        TeamsFailure::ActionTerminal { at, code, message } => TerminalError::new_with_code(
            *code,
            format!("teams stage remains incomplete after terminal action at {at}: {message}"),
        )
        .into(),
        TeamsFailure::IncompleteOutcome { at, outcome }
        | TeamsFailure::SourceFailures { at, outcome, .. } => TerminalError::new(format!(
            "teams stage remains incomplete at {at}: retained {} records with source errors: {}",
            outcome.records,
            outcome.errors.join("; ")
        ))
        .into(),
    }
}
