use census_domain::model::serialized_digest;
use census_store::{Store, StoreSnapshot};

use super::{read, terminal, AttemptOutcome, Completion, History, MAX_ATTEMPTS, PHASE};
use crate::restate_services::wire::{TeamsAttemptProgress, TeamsSourceOutcome};
use crate::restate_services::JobError;

pub(super) fn classify(
    attempt: u8,
    uncertain: bool,
    outcome: &AttemptOutcome,
    progress: &[TeamsAttemptProgress],
) -> Completion {
    match outcome {
        AttemptOutcome::Completed { outcome } => Completion::Settled(TeamsSourceOutcome::Completed {
            outcome:outcome.clone(), progress:progress.to_vec(),
        }),
        AttemptOutcome::Terminal { message, .. } => Completion::Settled(TeamsSourceOutcome::Terminal {
            message:message.clone(), progress:progress.to_vec(),
        }),
        AttemptOutcome::Transient { message, .. } if attempt == MAX_ATTEMPTS && uncertain => {
            Completion::Settled(TeamsSourceOutcome::Interrupted { attempts:Some(attempt),
                message:format!("budget consumed with uncertain acquisition outcomes; last recorded failure: {message}"),
                progress:progress.to_vec() })
        }
        AttemptOutcome::Transient { message, .. } if attempt == MAX_ATTEMPTS => {
            Completion::Settled(TeamsSourceOutcome::Exhausted {
                attempts:attempt, last_failure:message.clone(), progress:progress.to_vec(),
            })
        }
        AttemptOutcome::Transient { message, .. } => Completion::Retry(JobError::Transient { message:message.clone() }),
    }
}
pub(super) fn validated_settled(
    store: &StoreSnapshot<'_>,
    key: &str,
    history: &History,
    settle_uncertain: bool,
) -> Result<Option<TeamsSourceOutcome>, JobError> {
    let stored: Option<TeamsSourceOutcome> = read(store, &format!("{key}/settled"))?;
    let expected = match &history.settled {
        Some(outcome) => Some(outcome.clone()),
        None if history.attempt == MAX_ATTEMPTS
            && history.missing
            && (settle_uncertain || stored.is_some()) =>
        {
            Some(TeamsSourceOutcome::Interrupted {
                attempts: Some(MAX_ATTEMPTS),
                message: "teams source final reservation has no recorded outcome".to_string(),
                progress: history.progress.clone(),
            })
        }
        None => None,
    };
    if let Some(stored) = stored {
        let Some(expected) = &expected else {
            return Err(terminal(
                "settled teams source has no supporting attempt history",
            ));
        };
        if serialized_digest(&stored).map_err(|error| terminal(&error.to_string()))?
            != serialized_digest(expected).map_err(|error| terminal(&error.to_string()))?
        {
            return Err(terminal(
                "settled teams source contradicts its authoritative attempt history",
            ));
        }
    }
    Ok(expected)
}
pub(super) fn settle(
    store: &Store,
    key: &str,
    outcome: TeamsSourceOutcome,
) -> Result<TeamsSourceOutcome, JobError> {
    if !store.journal_contains(PHASE, &format!("{key}/settled"))? {
        let mut batch = store.write_batch();
        batch.journal_done(PHASE, &format!("{key}/settled"), &outcome)?;
        batch.commit()?;
    }
    Ok(outcome)
}
