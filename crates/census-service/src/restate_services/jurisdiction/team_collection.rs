use census_crawl::CollectionDisposition;
use futures::{stream, StreamExt, TryStreamExt};
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
    let selected = sources.iter().filter(|source| {
        source.as_str() != census_crawl::sidearm_staff::SOURCE_ID
            && teams_arms::arm_for(source).is_some()
    });
    let disposition = if selected.clone().next().is_some() {
        CollectionDisposition::Complete
    } else {
        CollectionDisposition::Unknown
    };
    let outcome = StageOutcome {
        records: 0,
        at: at.to_string(),
        errors: Vec::new(),
        notes: Vec::new(),
        disposition,
        unfinished: Vec::new(),
    };
    let (outcome, failures) = stream::iter(selected)
        .map(Ok::<_, HandlerError>)
        .try_fold(
            (outcome, Vec::new()),
            |(mut outcome, mut failures), source| async move {
                let input = source_request(request, source, at);
                retain(
                    source,
                    recovery::invoke(ctx, input).await,
                    &mut outcome,
                    &mut failures,
                )
                .map_err(job_error)?;
                Ok((outcome, failures))
            },
        )
        .await?;
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

fn source_request(request: &JurisdictionRequest, source: &str, at: &str) -> TeamsSourceRequest {
    TeamsSourceRequest {
        jurisdiction: request.clone(),
        source: source.to_string(),
        observed_on: request
            .observed_on
            .as_deref()
            .map_or(at, |value| value)
            .to_string(),
    }
}

pub(super) fn retain(
    source: &str,
    answer: TeamsSourceOutcome,
    aggregate: &mut StageOutcome,
    failures: &mut Vec<TeamsSourceFailure>,
) -> Result<(), JobError> {
    match answer {
        TeamsSourceOutcome::Completed { outcome, .. } => merge(source, outcome, aggregate),
        failure => {
            if let Some(outcome) = latest_progress(&failure) {
                merge(source, outcome.clone(), aggregate)?;
            }
            aggregate.disposition = CollectionDisposition::Partial;
            append(
                &mut aggregate.errors,
                format!("{source}: {}", failure_message(&failure)),
            )?;
            append(
                failures,
                TeamsSourceFailure {
                    source: source.to_string(),
                    outcome: failure,
                },
            )
        }
    }
}

fn merge(
    source: &str,
    outcome: StageOutcome,
    aggregate: &mut StageOutcome,
) -> Result<(), JobError> {
    aggregate.records = aggregate
        .records
        .checked_add(outcome.records)
        .ok_or_else(resource)?;
    if !outcome.disposition.is_complete()
        || !outcome.errors.is_empty()
        || !outcome.unfinished.is_empty()
    {
        aggregate.disposition = CollectionDisposition::Partial;
    }
    outcome
        .notes
        .into_iter()
        .try_for_each(|note| append(&mut aggregate.notes, format!("{source}: {note}")))?;
    outcome
        .errors
        .into_iter()
        .try_for_each(|error| append(&mut aggregate.errors, format!("{source}: {error}")))?;
    outcome
        .unfinished
        .into_iter()
        .try_for_each(|locator| append(&mut aggregate.unfinished, locator))
}

fn latest_progress(failure: &TeamsSourceOutcome) -> Option<&StageOutcome> {
    use crate::restate_services::wire::TeamsAttemptProgress;
    let progress = match failure {
        TeamsSourceOutcome::Completed { progress, .. }
        | TeamsSourceOutcome::Terminal { progress, .. }
        | TeamsSourceOutcome::Exhausted { progress, .. }
        | TeamsSourceOutcome::Interrupted { progress, .. } => progress,
    };
    progress.iter().rev().find_map(|step| match step {
        TeamsAttemptProgress::Completed { outcome, .. } => Some(outcome),
        TeamsAttemptProgress::Transient { outcome, .. }
        | TeamsAttemptProgress::Terminal { outcome, .. } => outcome.as_ref(),
        TeamsAttemptProgress::Unknown { .. } => None,
    })
}

fn failure_message(failure: &TeamsSourceOutcome) -> String {
    match failure {
        TeamsSourceOutcome::Terminal { message, .. } => format!("terminal: {message}"),
        TeamsSourceOutcome::Exhausted {
            attempts,
            last_failure,
            ..
        } => format!("exhausted after {attempts} attempts: {last_failure}"),
        TeamsSourceOutcome::Interrupted {
            attempts, message, ..
        } => format!("interrupted with observed attempts {attempts:?}: {message}"),
        TeamsSourceOutcome::Completed { .. } => "completed source in failure path".to_string(),
    }
}

fn append<T>(rows: &mut Vec<T>, value: T) -> Result<(), JobError> {
    if rows.len() >= 65536 {
        return Err(resource());
    }
    rows.try_reserve(1).map_err(|_| resource())?;
    rows.push(value);
    Ok(())
}

fn resource() -> JobError {
    JobError::Terminal {
        message: "teams aggregate capacity or counter exhausted".to_string(),
    }
}
