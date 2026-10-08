use crate::census::StateProgress;
use crate::restate_services::jobs;
use crate::restate_services::wire::{
    ConsolidatedTable, JurisdictionReport, JurisdictionRequest, JurisdictionState, TeamsStage,
};
use crate::restate_services::{HistoricalProgress, HistoryWindow, SourcePlan};
use census_reconcile::identity::WorkflowIdentity;
use restate_sdk::prelude::HandlerError;

struct ReadyReport {
    plan: SourcePlan,
    teams: usize,
    rosters: StateProgress,
    consolidated: Vec<ConsolidatedTable>,
    history: HistoricalProgress,
    window: HistoryWindow,
}

impl TryFrom<JurisdictionState> for ReadyReport {
    type Error = HandlerError;
    fn try_from(state: JurisdictionState) -> Result<Self, Self::Error> {
        let teams = team_records(&state.teams)?;
        Ok(Self {
            plan: required(state.plan, "source plan")?,
            teams,
            rosters: required(state.rosters, "roster outcome")?,
            consolidated: state
                .consolidated
                .map_or_else(Vec::new, core::convert::identity),
            history: state.history,
            window: required(state.history_window, "history window")?,
        })
    }
}

fn required<T>(value: Option<T>, subject: &str) -> Result<T, HandlerError> {
    value
        .ok_or_else(|| jobs::invariant(&format!("no {subject} recorded before report publication")))
}

fn team_records(stage: &TeamsStage) -> Result<usize, HandlerError> {
    match stage {
        TeamsStage::Completed(completed) => Ok(completed.outcome().records),
        TeamsStage::Failed(failure) => Err(super::teams_failure(failure)),
        TeamsStage::Owed => Err(jobs::invariant(
            "teams work remains owed after the stages ran",
        )),
    }
}

pub(super) fn report(
    request: &JurisdictionRequest,
    identity: &WorkflowIdentity,
    state: JurisdictionState,
    stages_run: Vec<String>,
    completed_at: String,
) -> Result<JurisdictionReport, HandlerError> {
    if !crate::restate_services::open_work::collection_is_complete(request.jurisdiction, &state)? {
        return Err(jobs::invariant(
            "jurisdiction source work remains incomplete after the stages ran",
        ));
    }
    let ready = ReadyReport::try_from(state)?;
    Ok(JurisdictionReport {
        identity: identity.as_str().to_string(),
        jurisdiction: request.jurisdiction,
        plan: ready.plan,
        stages_run,
        teams: ready.teams,
        rosters: ready.rosters,
        consolidated: ready.consolidated,
        history: ready.history,
        history_window: ready.window,
        completed_at,
    })
}
