use std::error::Error;

use super::{owed_jurisdictions, stages_of, JurisdictionState};
use crate::census::{MeetCensus, StateProgress};
use crate::restate_services::jurisdiction::roster_stage_owed;
use crate::restate_services::results_arms::{ResultsSourceRows, ResultsStageOutcome};
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
        meets_complete: true,
        results: Some(ResultsStageOutcome {
            per_source: vec![ResultsSourceRows {
                slug: "milesplit_results".to_string(),
                meets: 12,
                rows: 400,
                errors: 0,
                unresolved: None,
            }],
        }),
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

#[test]
fn results_failures_keep_the_jurisdiction_owed() -> Result<(), Box<dyn Error>> {
    let teams = TeamsStage::from_outcome(
        StageOutcome {
            records: 19,
            at: "2026-10-05".to_string(),
            errors: Vec::new(),
            notes: Vec::new(),
        },
        "2026-10-05".to_string(),
    );
    let clean = independent_stages_completed(teams);
    let failed = JurisdictionState {
        results: Some(ResultsStageOutcome {
            per_source: vec![ResultsSourceRows {
                slug: "milesplit_results".to_string(),
                meets: 12,
                rows: 400,
                errors: 1,
                unresolved: None,
            }],
        }),
        ..clean.clone()
    };
    let absent = JurisdictionState {
        results: None,
        ..clean.clone()
    };
    for (state, label) in [(&failed, "failed"), (&absent, "absent")] {
        let stages = stages_of(state);
        check!(!stages.results, "{label} results are not complete");
        check!(eq; stages.owed_results, 1);
        check!(!stages.terminal(), "{label} results stay owed");
        check!(eq; stages.owing(), vec!["results"]);
        check!(eq; owed_jurisdictions(&[stages]), 1);
    }
    let stages = stages_of(&clean);
    check!(stages.results);
    check!(eq; stages.owed_results, 0);
    check!(stages.terminal());
    Ok(())
}
