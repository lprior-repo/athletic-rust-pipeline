use restate_sdk::prelude::*;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;

use crate::census::{
    owed_jurisdictions, owed_source_objects, silent_source_objects, JurisdictionStages,
    SourceObject,
};
use census_reconcile::identity::{Revision, WorkflowIdentity};

use super::ingest::IngestClient;
use super::jurisdiction::JurisdictionCensusClient;
use super::wire::{
    JurisdictionOpen, JurisdictionState, OpenWorkReply, OpenWorkRequest, SourceObjectOpen,
};

pub(super) async fn measure(
    ctx: &Context<'_>,
    request: &OpenWorkRequest,
) -> Result<OpenWorkReply, HandlerError> {
    let season = SchoolYear::new(request.season).ok_or_else(|| {
        TerminalError::new(format!(
            "season year {} is not a school year",
            request.season
        ))
    })?;
    let revision = Revision(request.revision);
    let jurisdictions = read_jurisdictions(ctx, season, revision).await;
    let endpoints = read_source_objects(ctx, &request.source_objects).await;
    let source_objects = objects(&endpoints);
    Ok(OpenWorkReply {
        season: season.short(),
        revision: revision.get(),
        jurisdiction_sweeps: measured(&jurisdictions)
            .then(|| owed_jurisdictions(&stages(&jurisdictions))),
        source_objects: (!endpoints.is_empty() && measured(&endpoints))
            .then(|| owed_source_objects(&source_objects)),
        silent_sources: silent_source_objects(&source_objects),
        jurisdictions,
        endpoints,
    })
}

fn measured<T>(rows: &[T]) -> bool
where
    T: ReadRow,
{
    rows.iter().any(|row| !row.unreadable())
}

trait ReadRow {
    fn unreadable(&self) -> bool;
}

impl ReadRow for JurisdictionOpen {
    fn unreadable(&self) -> bool {
        self.unreadable
    }
}

impl ReadRow for SourceObjectOpen {
    fn unreadable(&self) -> bool {
        self.unreadable
    }
}

fn stages(rows: &[JurisdictionOpen]) -> Vec<JurisdictionStages> {
    rows.iter().map(|row| row.stages).collect()
}

fn objects(rows: &[SourceObjectOpen]) -> Vec<SourceObject> {
    rows.iter()
        .map(|row| SourceObject {
            endpoint: row.endpoint.clone(),
            observations: row.observations,
            windows: row.windows,
        })
        .collect()
}

async fn read_jurisdictions(
    ctx: &Context<'_>,
    season: SchoolYear,
    revision: Revision,
) -> Vec<JurisdictionOpen> {
    let scope = UsJurisdiction::CENSUS_SCOPE;
    let mut rows: Vec<JurisdictionOpen> = Vec::with_capacity(scope.len());
    let mut in_flight = DurableFuturesUnordered::new();
    for jurisdiction in scope {
        let identity = WorkflowIdentity::jurisdiction(jurisdiction, season, revision);
        let object = ctx.object_client::<JurisdictionCensusClient>(identity.as_str());
        in_flight.push(object.state().call());
        rows.push(JurisdictionOpen {
            jurisdiction,
            identity: identity.as_str().to_string(),
            stages: JurisdictionStages::default(),
            unreadable: true,
        });
    }
    while let Ok(Some((index, outcome))) = in_flight.next().await {
        let Ok(Json(state)) = outcome else {
            continue;
        };
        if let Some(row) = rows.get_mut(index) {
            row.stages = stages_of(&state);
            row.unreadable = false;
        }
    }
    rows
}

fn stages_of(state: &JurisdictionState) -> JurisdictionStages {
    JurisdictionStages {
        teams: state.teams.is_completed(),
        rosters: state
            .rosters
            .as_ref()
            .is_some_and(|progress| progress.is_terminal()),
        meets: state.meets.is_some(),
        owed_rosters: state.rosters.as_ref().map_or(0, |progress| {
            count(progress.rosters_remaining.max(progress.blocked_skipped))
        }),
    }
}

async fn read_source_objects(ctx: &Context<'_>, endpoints: &[String]) -> Vec<SourceObjectOpen> {
    let mut rows: Vec<SourceObjectOpen> = Vec::with_capacity(endpoints.len());
    let mut in_flight = DurableFuturesUnordered::new();
    for endpoint in endpoints {
        let object = ctx.object_client::<IngestClient>(endpoint.as_str());
        in_flight.push(object.state().call());
        rows.push(SourceObjectOpen {
            endpoint: endpoint.clone(),
            observations: 0,
            windows: 0,
            unreadable: true,
        });
    }
    while let Ok(Some((index, outcome))) = in_flight.next().await {
        let Ok(Json(state)) = outcome else {
            continue;
        };
        if let Some(row) = rows.get_mut(index) {
            row.observations = state.total_observations;
            row.windows = count(state.windows.len());
            row.unreadable = false;
        }
    }
    rows
}

fn count(value: usize) -> u64 {
    u64::try_from(value).map_or(u64::MAX, core::convert::identity)
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use super::{owed_jurisdictions, stages_of, JurisdictionState};
    use crate::census::{MeetCensus, StateProgress};
    use crate::restate_services::jurisdiction::roster_stage_owed;
    use crate::restate_services::wire::{StageOutcome, TeamsFailure, TeamsStage};
    use census_domain::UsJurisdiction;

    fn independent_stages_completed(teams: TeamsStage) -> JurisdictionState {
        JurisdictionState {
            teams,
            rosters: Some(StateProgress {
                jurisdiction: UsJurisdiction::Wisconsin,
                rosters_total: 2,
                rosters_committed: 2,
                rosters_remaining: 0,
                rosters_skipped: 0,
                athletes: 11,
                class_of_2027: 7,
                class_of_2027_boys: 3,
                class_of_2027_girls: 4,
                errors: Vec::new(),
                blocked: false,
                blocked_skipped: 0,
                teams: 2,
            }),
            meets: Some(MeetCensus::default()),
            ..JurisdictionState::default()
        }
    }

    #[test]
    fn roster_stage_owed_only_for_absent_or_nonterminal_progress() -> Result<(), Box<dyn Error>> {
        let absent = JurisdictionState::default();
        check!(roster_stage_owed(&absent));

        for (remaining, blocked, blocked_skipped, expected_owed) in [
            (2, false, 0, true),
            (0, true, 0, true),
            (0, false, 2, true),
            (0, false, 0, false),
        ] {
            let mut state = independent_stages_completed(TeamsStage::Owed);
            let progress = state.rosters.as_mut().ok_or("missing roster fixture")?;
            progress.rosters_remaining = remaining;
            progress.blocked = blocked;
            progress.blocked_skipped = blocked_skipped;

            check!(eq; roster_stage_owed(&state), expected_owed);
        }
        Ok(())
    }

    #[test]
    fn failed_teams_remain_owed_without_automatic_work() -> Result<(), Box<dyn Error>> {
        let terminal = TeamsStage::Failed(TeamsFailure::ActionTerminal {
            at: "2026-10-01".to_string(),
            code: 503,
            message: "coach source unavailable".to_string(),
        });
        let partial = TeamsStage::from_outcome(
            StageOutcome {
                records: 19,
                at: "2026-10-01".to_string(),
                errors: vec!["coach source incomplete".to_string()],
                notes: Vec::new(),
            },
            "2026-10-01".to_string(),
        );
        [terminal, partial].into_iter().try_for_each(|teams| {
            let state = independent_stages_completed(teams);
            let restored: JurisdictionState = serde_json::from_value(serde_json::to_value(state)?)?;
            let stages = stages_of(&restored);

            check!(!restored.teams.is_owed());
            check!(!stages.terminal());
            check!(eq; stages.owing(), vec!["teams"]);
            check!(eq; owed_jurisdictions(&[stages]), 1);
            Ok::<(), Box<dyn Error>>(())
        })
    }

    #[test]
    fn completed_teams_allow_jurisdiction_completion() -> Result<(), Box<dyn Error>> {
        let teams = TeamsStage::from_outcome(
            StageOutcome {
                records: 19,
                at: "2026-10-01".to_string(),
                errors: Vec::new(),
                notes: Vec::new(),
            },
            "2026-10-01".to_string(),
        );
        let state = independent_stages_completed(teams);
        let restored: JurisdictionState = serde_json::from_value(serde_json::to_value(state)?)?;
        let stages = stages_of(&restored);

        check!(stages.terminal());
        check!(eq; stages.owing(), Vec::<&str>::new());
        check!(eq; owed_jurisdictions(&[stages]), 0);
        Ok(())
    }

    #[test]
    fn unfinished_rosters_keep_the_jurisdiction_nonterminal() -> Result<(), Box<dyn Error>> {
        let teams = TeamsStage::from_outcome(
            StageOutcome {
                records: 19,
                at: "2026-10-03".to_string(),
                errors: Vec::new(),
                notes: Vec::new(),
            },
            "2026-10-03".to_string(),
        );
        for (remaining, blocked, blocked_skipped, expected_owed) in
            [(2, false, 0, 2), (2, true, 2, 2), (0, true, 0, 0)]
        {
            let mut state = independent_stages_completed(teams.clone());
            let progress = state.rosters.as_mut().ok_or("missing roster fixture")?;
            progress.rosters_total = if remaining == 0 { 2 } else { 4 };
            progress.rosters_remaining = remaining;
            progress.blocked = blocked;
            progress.blocked_skipped = blocked_skipped;
            let stages = stages_of(&state);

            check!(eq; stages.rosters, false);
            check!(eq; stages.owed_rosters, expected_owed);
            check!(!stages.terminal());
            check!(eq; owed_jurisdictions(&[stages]), 1);
        }
        Ok(())
    }
}
