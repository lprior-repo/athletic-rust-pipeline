use crate::restate_services::wire::{StageOutcome, TeamsAttemptProgress};
use crate::restate_services::JobError;

use super::{terminal, AttemptOutcome};

pub(crate) enum Attempt {
    Completed(StageOutcome),
    Incomplete {
        outcome: StageOutcome,
        error: JobError,
    },
    Failed(JobError),
}

impl From<Result<StageOutcome, JobError>> for Attempt {
    fn from(result: Result<StageOutcome, JobError>) -> Self {
        match result {
            Ok(outcome) => Self::Completed(outcome),
            Err(error) => Self::Failed(error),
        }
    }
}

impl Attempt {
    pub(super) fn into_outcome(self, observed_on: &str) -> Result<AttemptOutcome, JobError> {
        let outcome = match self {
            Self::Completed(outcome) => AttemptOutcome::Completed { outcome },
            Self::Incomplete { outcome, error } => failure(error, Some(outcome)),
            Self::Failed(error) => failure(error, None),
        };
        validate(&outcome, observed_on)?;
        Ok(outcome)
    }
}

fn failure(error: JobError, progress: Option<StageOutcome>) -> AttemptOutcome {
    match error {
        JobError::Transient { message } => AttemptOutcome::Transient { message, progress },
        JobError::Terminal { message } => AttemptOutcome::Terminal { message, progress },
    }
}

pub(super) fn validate(outcome: &AttemptOutcome, observed_on: &str) -> Result<(), JobError> {
    let valid = match outcome {
        AttemptOutcome::Completed { outcome } => complete(outcome) && outcome.at == observed_on,
        AttemptOutcome::Transient {
            progress: Some(outcome),
            ..
        }
        | AttemptOutcome::Terminal {
            progress: Some(outcome),
            ..
        } => !complete(outcome) && outcome.at == observed_on,
        AttemptOutcome::Transient { progress: None, .. }
        | AttemptOutcome::Terminal { progress: None, .. } => true,
    };
    if valid {
        Ok(())
    } else {
        Err(terminal(
            "teams source outcome has inconsistent progress or date",
        ))
    }
}

fn complete(outcome: &StageOutcome) -> bool {
    outcome.disposition.is_complete() && outcome.errors.is_empty() && outcome.unfinished.is_empty()
}

pub(super) fn progress(attempt: u8, outcome: &AttemptOutcome) -> TeamsAttemptProgress {
    match outcome {
        AttemptOutcome::Completed { outcome } => TeamsAttemptProgress::Completed {
            attempt,
            outcome: outcome.clone(),
        },
        AttemptOutcome::Transient { message, progress } => TeamsAttemptProgress::Transient {
            attempt,
            outcome: progress.clone(),
            message: message.clone(),
        },
        AttemptOutcome::Terminal { message, progress } => TeamsAttemptProgress::Terminal {
            attempt,
            outcome: progress.clone(),
            message: message.clone(),
        },
    }
}
