use census_store::Store;

use super::settlement::{settle, validated_settled};
use super::{history, identity, next_attempt, read, slot_key, terminal, Identity, Reservation};
use super::{MAX_ATTEMPTS, PHASE};
use crate::restate_services::wire::{TeamsSourceOutcome, TeamsSourceRequest};
use crate::restate_services::JobError;

pub(in crate::restate_services::jurisdiction::team_source) enum AdmissionReplay {
    Settled(TeamsSourceOutcome),
    Reserved { attempt: u8, observed_on: String },
    ReplayRetry { message: String },
}

pub(in crate::restate_services::jurisdiction::team_source) fn baseline(
    store: &Store,
    key: &str,
) -> Result<u8, JobError> {
    let snapshot = store.snapshot();
    let identity: Identity = read(&snapshot, &format!("{key}/identity"))?
        .ok_or_else(|| terminal("teams source baseline has no registered identity"))?;
    let history = history::read(&snapshot, key, &identity)?;
    Ok(history.attempt)
}

pub(in crate::restate_services::jurisdiction::team_source) fn begin_at(
    store: &Store,
    key: &str,
    request: &TeamsSourceRequest,
    registered: &Identity,
    baseline: u8,
) -> Result<AdmissionReplay, JobError> {
    let snapshot = store.snapshot();
    let (identity, initial) = identity(&snapshot, key, request)?;
    if initial {
        return Err(terminal(
            "teams source admission has no registered identity",
        ));
    }
    if identity.request_digest != registered.request_digest
        || identity.observed_on != registered.observed_on
    {
        return Err(terminal(
            "teams source registration contradicts persisted identity",
        ));
    }
    let history = history::read(&snapshot, key, &identity)?;
    let planned = planned_slot(baseline, history.attempt)?;
    if let Some(outcome) = validated_settled(&snapshot, key, &history, true)? {
        return settle(store, key, outcome).map(AdmissionReplay::Settled);
    }
    if history.attempt == planned {
        if history.missing {
            return Ok(AdmissionReplay::ReplayRetry {
                message: format!(
                    "teams source attempt {} reserved with no recorded outcome; acquisition remains uncertain",
                    history.attempt
                ),
            });
        }
        let message = history
            .transient
            .ok_or_else(|| terminal("teams source replay has no recorded transient outcome"))?;
        return Ok(AdmissionReplay::ReplayRetry { message });
    }
    reserve(store, key, identity, next_attempt(history.attempt)?)
}

fn planned_slot(baseline: u8, actual: u8) -> Result<u8, JobError> {
    let planned = baseline
        .checked_add(1)
        .filter(|_| baseline <= MAX_ATTEMPTS)
        .ok_or_else(|| terminal("teams source baseline is outside the persisted budget"))?;
    if actual < baseline {
        return Err(terminal(
            "teams source attempt history regressed below baseline",
        ));
    }
    if actual > planned {
        return Err(terminal(
            "teams source attempt history exceeds baseline plus one",
        ));
    }
    Ok(planned)
}

fn reserve(
    store: &Store,
    key: &str,
    identity: Identity,
    attempt: u8,
) -> Result<AdmissionReplay, JobError> {
    let reservation = Reservation {
        attempt,
        observed_on: identity.observed_on.clone(),
    };
    let mut batch = store.write_batch();
    batch.journal_done(PHASE, &slot_key(key, attempt, "reserved"), &reservation)?;
    batch.commit()?;
    Ok(AdmissionReplay::Reserved {
        attempt,
        observed_on: identity.observed_on,
    })
}
