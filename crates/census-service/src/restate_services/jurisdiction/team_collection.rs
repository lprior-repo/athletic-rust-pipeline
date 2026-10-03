use restate_sdk::prelude::*;

mod recovery;
use crate::restate_services::wire::{
    JurisdictionRequest, StageOutcome, TeamsFailure, TeamsSourceFailure, TeamsSourceOutcome,
    TeamsSourceRequest, TeamsStage,
};
use crate::restate_services::{job_error, teams_arms, JobError};

pub(super) async fn collect(
    ctx: &ObjectContext<'_>,
    request: &JurisdictionRequest,
    sources: &[String],
    at: &str,
) -> Result<TeamsStage, HandlerError> {
    let mut outcome = StageOutcome {
        records: 0,
        at: at.to_string(),
        errors: Vec::new(),
        notes: Vec::new(),
    };
    let mut failures = Vec::new();
    for source in sources
        .iter()
        .filter(|source| teams_arms::arm_for(source).is_some())
    {
        let input = TeamsSourceRequest {
            jurisdiction: request.clone(),
            source: source.clone(),
            observed_on: request
                .observed_on
                .as_deref()
                .map_or(at, |value| value)
                .to_string(),
        };
        let answer = recovery::invoke(ctx, input).await;
        retain(source, answer, &mut outcome, &mut failures).map_err(job_error)?;
    }
    Ok(if failures.is_empty() {
        TeamsStage::from_outcome(outcome, at.to_string())
    } else {
        TeamsStage::Failed(TeamsFailure::SourceFailures {
            at: at.to_string(),
            outcome,
            failures,
        })
    })
}

pub(super) fn retain(
    source: &str,
    answer: TeamsSourceOutcome,
    aggregate: &mut StageOutcome,
    failures: &mut Vec<TeamsSourceFailure>,
) -> Result<(), JobError> {
    match answer {
        TeamsSourceOutcome::Completed { outcome, .. } => {
            aggregate.records =
                aggregate
                    .records
                    .checked_add(outcome.records)
                    .ok_or_else(|| JobError::Terminal {
                        message: "teams source record total overflow".to_string(),
                    })?;
            aggregate.notes.extend(
                outcome
                    .notes
                    .into_iter()
                    .map(|note| format!("{source}: {note}")),
            );
            aggregate.errors.extend(
                outcome
                    .errors
                    .into_iter()
                    .map(|error| format!("{source}: {error}")),
            );
        }
        failure => {
            let message = match &failure {
                TeamsSourceOutcome::Terminal { message, .. } => format!("terminal: {message}"),
                TeamsSourceOutcome::Exhausted {
                    attempts,
                    last_failure,
                    ..
                } => {
                    format!("exhausted after {attempts} attempts: {last_failure}")
                }
                TeamsSourceOutcome::Interrupted {
                    attempts, message, ..
                } => {
                    format!("interrupted with observed attempts {attempts:?}: {message}")
                }
                TeamsSourceOutcome::Completed { .. } => {
                    return Err(JobError::Terminal {
                        message: "completed teams source entered failure retention".to_string(),
                    });
                }
            };
            aggregate.errors.push(format!("{source}: {message}"));
            failures.push(TeamsSourceFailure {
                source: source.to_string(),
                outcome: failure,
            });
        }
    }
    Ok(())
}
