use super::*;
use crate::restate_services::jurisdiction::team_collection;
use crate::restate_services::wire::{
    SourcePlan, StageOutcome, TeamsFailure, TeamsSourceOutcome, TeamsStage,
};
use crate::restate_services::JobError;
use census_crawl::CollectionDisposition;
use census_reconcile::identity::WorkflowIdentity;

pub(super) fn seed(
    store: &Store,
    input: &TeamsSourceRequest,
    mode: &str,
) -> TestResult<(JurisdictionState, Option<TeamsContinuationOutcome>)> {
    let mut ihsa = input.clone();
    ihsa.source = "ihsa".into();
    let first = ledger::Attempt::Incomplete {
        outcome: stage(3, false, "retained IHSA first rows"),
        error: JobError::Transient {
            message: "retained IHSA timeout".into(),
        },
    };
    check!(matches!(
        attempt(store, &ihsa, first)?,
        ledger::Completion::Retry(_)
    ));
    let result = if mode == "settled_success" {
        ledger::Attempt::Completed(stage(7, true, "retained IHSA success"))
    } else {
        ledger::Attempt::Incomplete {
            outcome: stage(5, false, "retained IHSA terminal rows"),
            error: JobError::Terminal {
                message: "retained IHSA refusal".into(),
            },
        }
    };
    let ledger::Completion::Settled(ihsa) = attempt(store, &ihsa, result)? else {
        return Err("IHSA did not settle".into());
    };
    let holder = serde_json::json!({"origin":"https://oh.milesplit.com", "pid":12345});
    let message = format!("origin https://oh.milesplit.com is held by another census-service process (holder: {holder})");
    let ledger::Completion::Settled(original) = attempt(
        store,
        input,
        ledger::Attempt::Failed(JobError::Terminal { message }),
    )?
    else {
        return Err("original source did not settle".into());
    };
    let original_outcome_digest = serialized_digest(&original)?;
    let authorization = ledger::origin_continuation::authorize(
        store,
        &key(input),
        input,
        &original_outcome_digest,
    )?;
    let current = continue_source(store, input, &authorization, mode)?;
    let state = parent(input, ihsa, original)?;
    Ok((
        state,
        current.map(|outcome| TeamsContinuationOutcome {
            original_outcome_digest,
            outcome,
        }),
    ))
}

fn continue_source(
    store: &Store,
    input: &TeamsSourceRequest,
    authorization: &ledger::origin_continuation::Authorization,
    mode: &str,
) -> TestResult<Option<TeamsSourceOutcome>> {
    if mode == "pending" {
        return Ok(None);
    }
    let ledger::AdmissionReplay::Reserved { attempt, .. } =
        ledger::origin_continuation::begin_at(store, &key(input), input, authorization, 1)?
    else {
        return Err("continuation was not admitted".into());
    };
    check!(eq; attempt, 2);
    let result = if mode == "continued_failure" {
        ledger::Attempt::Failed(JobError::Terminal {
            message: "continued source refused".into(),
        })
    } else {
        ledger::Attempt::Completed(stage(2, true, "continued directory"))
    };
    let ledger::Completion::Settled(outcome) =
        ledger::origin_continuation::finish(store, &key(input), attempt, result, &[])?
    else {
        return Err("continuation did not settle".into());
    };
    Ok(Some(outcome))
}

fn attempt(
    store: &Store,
    input: &TeamsSourceRequest,
    result: ledger::Attempt,
) -> TestResult<ledger::Completion> {
    let (attempt, _) = super::super::super::reserve(store, input)?;
    Ok(ledger::finish(store, &key(input), attempt, result, &[])?)
}

fn parent(
    input: &TeamsSourceRequest,
    ihsa: TeamsSourceOutcome,
    original: TeamsSourceOutcome,
) -> TestResult<JurisdictionState> {
    let mut aggregate =
        team_collection::empty_outcome(&input.observed_on, CollectionDisposition::Complete);
    let mut failures = Vec::new();
    for (source, outcome) in [("ihsa", ihsa), ("milesplit", original)] {
        team_collection::retain(source, outcome, &mut aggregate, &mut failures)?;
    }
    Ok(JurisdictionState {
        identity: WorkflowIdentity::jurisdiction(
            input.jurisdiction.jurisdiction,
            input.jurisdiction.season,
            input.jurisdiction.revision,
        )
        .to_string(),
        plan: Some(SourcePlan {
            sweepable: vec!["ihsa".into(), "milesplit".into()],
            fingerprint: public_resources_fixture::fingerprint(input),
            ..SourcePlan::default()
        }),
        teams: team_collection::finish(aggregate, failures, &input.observed_on),
        rosters: Some(crate::census::StateProgress {
            jurisdiction: input.jurisdiction.jurisdiction,
            rosters_total: 9,
            rosters_committed: 4,
            rosters_remaining: 5,
            rosters_skipped: 0,
            athletes: 24,
            class_of_2027: 6,
            class_of_2027_boys: 3,
            class_of_2027_girls: 3,
            errors: vec!["retained unrelated roster diagnostic".into()],
            blocked: true,
            blocked_skipped: 2,
            teams: 9,
        }),
        history_window: Some(input.jurisdiction.history),
        updated_at: Some("2026-10-02".into()),
        ..JurisdictionState::default()
    })
}

pub(super) fn alter_parent(state: &mut JurisdictionState) -> TestResult {
    let TeamsStage::Failed(TeamsFailure::SourceFailures { failures, .. }) = &mut state.teams else {
        return Err("source failure absent".into());
    };
    let original = failures
        .iter_mut()
        .find(|failure| failure.source == "milesplit")
        .ok_or("MS absent")?;
    let TeamsSourceOutcome::Terminal { message, .. } = &mut original.outcome else {
        return Err("original terminal absent".into());
    };
    message.push_str(" unrelated parent mutation");
    Ok(())
}

fn stage(records: usize, complete: bool, note: &str) -> StageOutcome {
    StageOutcome {
        records,
        at: "2026-10-02".into(),
        errors: Vec::new(),
        notes: vec![note.into()],
        disposition: if complete {
            CollectionDisposition::Complete
        } else {
            CollectionDisposition::Partial
        },
        unfinished: Vec::new(),
    }
}
