use census_store::StoreSnapshot;

use super::{classify, next_attempt, read as read_payload, slot_key, terminal};
use super::{AttemptOutcome, Completion, History, Identity, Reservation, MAX_ATTEMPTS};
use crate::restate_services::wire::TeamsAttemptProgress;
use crate::restate_services::JobError;

pub(super) fn read(
    store: &StoreSnapshot<'_>,
    key: &str,
    identity: &Identity,
) -> Result<History, JobError> {
    let mut history = History::default();
    for attempt in 1..=MAX_ATTEMPTS {
        let reservation: Option<Reservation> =
            read_payload(store, &slot_key(key, attempt, "reserved"))?;
        let outcome: Option<AttemptOutcome> =
            read_payload(store, &slot_key(key, attempt, "outcome"))?;
        let Some(reservation) = reservation else {
            if outcome.is_some() {
                return Err(terminal("teams source outcome has no reservation"));
            }
            continue;
        };
        if history.settled.is_some() {
            return Err(terminal(
                "teams source has additional slots after settlement",
            ));
        }
        if next_attempt(history.attempt)? != attempt
            || reservation.attempt != attempt
            || reservation.observed_on != identity.observed_on
        {
            return Err(terminal(
                "teams source reservation metadata mismatch or noncontiguous slots",
            ));
        }
        history.uncertain |= history.missing;
        history.attempt = attempt;
        history.missing = outcome.is_none();
        history.transient = None;
        let progress = outcome
            .as_ref()
            .map(|outcome| super::attempt::progress(attempt, outcome))
            .map_or(TeamsAttemptProgress::Unknown { attempt }, |value| value);
        history.progress.push(progress);
        if let Some(outcome) = outcome {
            super::attempt::validate(&outcome, &identity.observed_on)?;
            match classify(attempt, history.uncertain, &outcome, &history.progress) {
                Completion::Settled(outcome) => history.settled = Some(outcome),
                Completion::Retry(JobError::Transient { message }) => {
                    history.transient = Some(message)
                }
                Completion::Retry(JobError::Terminal { .. }) => {
                    return Err(terminal("terminal teams source outcome cannot be retried"));
                }
            }
        }
    }
    Ok(history)
}
