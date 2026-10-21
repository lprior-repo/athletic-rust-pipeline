use super::{
    attempt, settlement, terminal, AdmissionReplay, Attempt, AttemptOutcome, Completion, Identity,
    Reservation, MAX_ATTEMPTS,
};
use crate::restate_services::wire::{TeamsAttemptProgress, TeamsSourceOutcome, TeamsSourceRequest};
use crate::restate_services::JobError;
use census_domain::model::{serialized_digest, SourceAccessCondition};
use census_store::{Store, StoreSnapshot};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

mod authorization;
mod history;
pub(in crate::restate_services::jurisdiction::team_source) use authorization::{
    authorize, validate, Authorization,
};
pub(in crate::restate_services::jurisdiction::team_source) const PHASE: &str =
    "teams_source_origin_continuation_v1";

struct State {
    authorization: Authorization,
    attempt: u8,
    missing: bool,
    uncertain: bool,
    progress: Vec<TeamsAttemptProgress>,
    settled: Option<TeamsSourceOutcome>,
    transient: Option<String>,
}

pub(in crate::restate_services::jurisdiction::team_source) fn status(
    store: &Store,
    key: &str,
) -> Result<Option<TeamsSourceOutcome>, JobError> {
    history::read(&store.snapshot(), key).map(|state| state.and_then(|state| state.settled))
}

pub(in crate::restate_services::jurisdiction::team_source) fn baseline(
    store: &Store,
    key: &str,
) -> Result<u8, JobError> {
    Ok(required(&store.snapshot(), key)?.attempt)
}

pub(in crate::restate_services::jurisdiction::team_source) fn begin_at(
    store: &Store,
    key: &str,
    request: &TeamsSourceRequest,
    authorization: &Authorization,
    baseline: u8,
) -> Result<AdmissionReplay, JobError> {
    let snapshot = store.snapshot();
    let state = required(&snapshot, key)?;
    if &state.authorization != authorization
        || validate(&snapshot, key, Some(request))? != *authorization
    {
        return Err(terminal("origin continuation authorization mismatch"));
    }
    let planned = baseline
        .checked_add(1)
        .filter(|_| baseline <= MAX_ATTEMPTS)
        .ok_or_else(|| terminal("origin continuation baseline is outside the global budget"))?;
    if state.attempt < baseline || state.attempt > planned {
        return Err(terminal(
            "origin continuation history does not match its baseline",
        ));
    }
    if let Some(outcome) = final_outcome(&state) {
        if !store.journal_contains(PHASE, &format!("{key}/settled"))? {
            let mut batch = store.write_batch();
            batch.journal_done(PHASE, &format!("{key}/settled"), &outcome)?;
            batch.commit()?;
        }
        return Ok(AdmissionReplay::Settled(outcome));
    }
    if state.attempt == planned {
        return Ok(AdmissionReplay::ReplayRetry {
            message: match state.transient {
                Some(message) => message,
                None => format!(
                    "origin continuation attempt {} reserved without a recorded outcome",
                    state.attempt
                ),
            },
        });
    }
    let attempt = super::next_attempt(state.attempt)?;
    let observed_on = authorization.observed_on.clone();
    let mut batch = store.write_batch();
    batch.journal_done(
        PHASE,
        &slot_key(key, attempt, "reserved"),
        &Reservation {
            attempt,
            observed_on: observed_on.clone(),
        },
    )?;
    batch.commit()?;
    Ok(AdmissionReplay::Reserved {
        attempt,
        observed_on,
    })
}

pub(in crate::restate_services::jurisdiction::team_source) fn finish(
    store: &Store,
    key: &str,
    attempt: u8,
    result: Attempt,
    conditions: &[SourceAccessCondition],
) -> Result<Completion, JobError> {
    let mut state = required(&store.snapshot(), key)?;
    if attempt <= state.authorization.original_attempt || attempt > MAX_ATTEMPTS {
        return Err(terminal(
            "origin continuation completion is outside its remaining budget",
        ));
    }
    if let Some(outcome) = state.settled {
        return Ok(Completion::Settled(outcome));
    }
    if state.attempt != attempt {
        return Err(terminal(
            "origin continuation completion has no matching reservation",
        ));
    }
    if let Some(message) = state.transient {
        return Ok(Completion::Retry(JobError::Transient { message }));
    }
    if !state.missing {
        return Err(terminal(
            "origin continuation completion has no outstanding reservation",
        ));
    }
    let (outcome, resource) = result.into_outcome(&state.authorization.observed_on)?;
    let last = state
        .progress
        .last_mut()
        .ok_or_else(|| terminal("continuation progress missing"))?;
    *last = attempt::progress(attempt, &outcome);
    let completion = settlement::classify(attempt, state.uncertain, &outcome, &state.progress);
    let persisted = (|| -> census_store::StoreResult<()> {
        let mut batch = store.write_batch();
        crate::census::access::stage(&mut batch, conditions)?;
        batch.journal_done(PHASE, &slot_key(key, attempt, "outcome"), &outcome)?;
        if let Completion::Settled(outcome) = &completion {
            batch.journal_done(PHASE, &format!("{key}/settled"), outcome)?;
        }
        batch.commit()?;
        Ok(())
    })();
    if let Err(secondary) = persisted {
        return Err(super::completion::persistence_failure(
            &outcome, resource, secondary,
        ));
    }
    Ok(completion)
}

fn required(snapshot: &StoreSnapshot<'_>, key: &str) -> Result<State, JobError> {
    history::read(snapshot, key)?.ok_or_else(|| terminal("origin continuation is not authorized"))
}

fn final_outcome(state: &State) -> Option<TeamsSourceOutcome> {
    state.settled.clone().or_else(|| {
        (state.missing && state.attempt == MAX_ATTEMPTS).then(|| TeamsSourceOutcome::Interrupted {
            attempts: Some(MAX_ATTEMPTS),
            message: "origin continuation final reservation has no recorded outcome".into(),
            progress: state.progress.clone(),
        })
    })
}

pub(super) fn attempt_history(
    snapshot: &StoreSnapshot<'_>,
    key: &str,
) -> Result<Option<(u8, Vec<TeamsAttemptProgress>, bool)>, JobError> {
    history::read(snapshot, key)
        .map(|state| state.map(|state| (state.attempt, state.progress, state.settled.is_some())))
}

fn read<T: DeserializeOwned>(
    snapshot: &StoreSnapshot<'_>,
    key: &str,
) -> Result<Option<T>, JobError> {
    snapshot
        .journal_payload(PHASE, key)?
        .map(|value| {
            serde_json::from_value(value)
                .map_err(|error| terminal(&format!("decoding origin continuation: {error}")))
        })
        .transpose()
}

fn slot_key(key: &str, attempt: u8, suffix: &str) -> String {
    format!("{key}/attempt/{attempt}/{suffix}")
}

#[cfg(test)]
mod adversarial_tests;
#[cfg(test)]
mod final_reservation_tests;
#[cfg(test)]
mod remaining_budget_tests;
#[cfg(test)]
mod tests;
