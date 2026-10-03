use census_domain::model::serialized_digest;
use census_reconcile::identity::WorkflowIdentity;
use census_store::{Store, StoreSnapshot};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::restate_services::support::JobError;
use crate::restate_services::wire::{
    StageOutcome, TeamsAttemptProgress, TeamsSourceInspection, TeamsSourceOutcome,
    TeamsSourceRequest,
};

mod attempt;
mod history;
mod registration;
mod settlement;
pub(super) use attempt::Attempt;
pub(super) use registration::register;
use settlement::{classify, settle, validated_settled};

pub(super) const PHASE: &str = "teams_source_attempts_v1";
const MAX_ATTEMPTS: u8 = 3;

pub(super) enum Admission {
    Settled(TeamsSourceOutcome),
    Reserved { attempt: u8, observed_on: String },
}

pub(super) enum Completion {
    Settled(TeamsSourceOutcome),
    Retry(JobError),
}

#[derive(Serialize, Deserialize)]
pub(super) struct Identity {
    pub(super) request_digest: String,
    pub(super) observed_on: String,
}

#[derive(Serialize, Deserialize)]
struct Reservation {
    attempt: u8,
    observed_on: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum AttemptOutcome {
    Completed {
        outcome: StageOutcome,
    },
    Terminal {
        message: String,
        progress: Option<StageOutcome>,
    },
    Transient {
        message: String,
        progress: Option<StageOutcome>,
    },
}

#[derive(Default)]
struct History {
    attempt: u8,
    uncertain: bool,
    missing: bool,
    transient: Option<String>,
    settled: Option<TeamsSourceOutcome>,
    progress: Vec<TeamsAttemptProgress>,
}

pub(super) fn begin(
    store: &Store,
    key: &str,
    request: &TeamsSourceRequest,
    registered: &Identity,
) -> Result<Admission, JobError> {
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
    if let Some(outcome) = validated_settled(&snapshot, key, &history, true)? {
        return settle(store, key, outcome).map(Admission::Settled);
    }
    let attempt = next_attempt(history.attempt)?;
    let reservation = Reservation {
        attempt,
        observed_on: identity.observed_on.clone(),
    };
    let mut batch = store.write_batch();
    batch.journal_done(PHASE, &slot_key(key, attempt, "reserved"), &reservation)?;
    batch.commit()?;
    Ok(Admission::Reserved {
        attempt,
        observed_on: identity.observed_on,
    })
}

pub(super) fn status(
    store: &Store,
    key: &str,
    request: Option<&TeamsSourceRequest>,
) -> Result<Option<TeamsSourceOutcome>, JobError> {
    inspection(store, key, request).map(|inspection| match inspection {
        TeamsSourceInspection::Settled { outcome } => Some(outcome),
        TeamsSourceInspection::Unsettled { .. } => None,
    })
}

pub(super) fn inspection(
    store: &Store,
    key: &str,
    request: Option<&TeamsSourceRequest>,
) -> Result<TeamsSourceInspection, JobError> {
    let snapshot = store.snapshot();
    let stored: Option<Identity> = read(&snapshot, &format!("{key}/identity"))?;
    if let Some(request) = request {
        let expected = request_digest(key, request)?;
        if stored
            .as_ref()
            .is_some_and(|identity| identity.request_digest != expected)
        {
            return Err(terminal("teams source request digest mismatch"));
        }
    }
    let absent = stored.is_none();
    let identity = stored.map_or(
        Identity {
            request_digest: String::new(),
            observed_on: String::new(),
        },
        |value| value,
    );
    let history = history::read(&snapshot, key, &identity)?;
    if absent && history.attempt != 0 {
        return Err(terminal("teams source attempt history has no identity"));
    }
    let outcome = validated_settled(&snapshot, key, &history, false)?;
    Ok(match outcome {
        Some(outcome) => TeamsSourceInspection::Settled { outcome },
        None => TeamsSourceInspection::Unsettled {
            progress: history.progress,
        },
    })
}

pub(super) fn finish(
    store: &Store,
    key: &str,
    attempt: u8,
    result: Attempt,
) -> Result<Completion, JobError> {
    if !(1..=MAX_ATTEMPTS).contains(&attempt) {
        return Err(terminal(
            "teams source attempt is outside the persisted budget",
        ));
    }
    let snapshot = store.snapshot();
    let identity: Identity = read(&snapshot, &format!("{key}/identity"))?
        .ok_or_else(|| terminal("teams source completion has no operation identity"))?;
    let mut history = history::read(&snapshot, key, &identity)?;
    if let Some(outcome) = validated_settled(&snapshot, key, &history, false)? {
        return settle(store, key, outcome).map(Completion::Settled);
    }
    if history.attempt != attempt {
        return Err(terminal(
            "teams source completion does not match the latest reservation",
        ));
    }
    if let Some(message) = history.transient {
        return Ok(Completion::Retry(JobError::Transient { message }));
    }
    if !history.missing {
        return Err(terminal(
            "teams source completion has no outstanding reservation",
        ));
    }
    let outcome = result.into_outcome(&identity.observed_on)?;
    let recorded = history
        .progress
        .last_mut()
        .ok_or_else(|| terminal("teams source reservation has no progress slot"))?;
    *recorded = attempt::progress(attempt, &outcome);
    let completion = classify(attempt, history.uncertain, &outcome, &history.progress);
    let mut batch = store.write_batch();
    batch.journal_done(PHASE, &slot_key(key, attempt, "outcome"), &outcome)?;
    if let Completion::Settled(outcome) = &completion {
        batch.journal_done(PHASE, &format!("{key}/settled"), outcome)?;
    }
    batch.commit()?;
    Ok(completion)
}

fn identity(
    store: &StoreSnapshot<'_>,
    key: &str,
    request: &TeamsSourceRequest,
) -> Result<(Identity, bool), JobError> {
    let expected = request_digest(key, request)?;
    match read::<Identity>(store, &format!("{key}/identity"))? {
        Some(identity) if identity.request_digest == expected => Ok((identity, false)),
        Some(_) => Err(terminal("teams source request digest mismatch")),
        None => Ok((
            Identity {
                request_digest: expected,
                observed_on: request.observed_on.clone(),
            },
            true,
        )),
    }
}

fn request_digest(key: &str, request: &TeamsSourceRequest) -> Result<String, JobError> {
    let jurisdiction = &request.jurisdiction;
    let identity = WorkflowIdentity::jurisdiction(
        jurisdiction.jurisdiction,
        jurisdiction.season,
        jurisdiction.revision,
    );
    if key != format!("{}/teams/{}", identity.as_str(), request.source) {
        return Err(terminal(
            "teams source request identity does not match object key",
        ));
    }
    serialized_digest(&(
        jurisdiction.jurisdiction,
        jurisdiction.season,
        jurisdiction.revision,
        &request.source,
    ))
    .map_err(|error| terminal(&format!("digesting teams source identity: {error}")))
}

fn read<T: DeserializeOwned>(store: &StoreSnapshot<'_>, key: &str) -> Result<Option<T>, JobError> {
    store
        .journal_payload(PHASE, key)?
        .map(|payload| {
            serde_json::from_value(payload)
                .map_err(|error| terminal(&format!("decoding teams source journal {key}: {error}")))
        })
        .transpose()
}

fn next_attempt(previous: u8) -> Result<u8, JobError> {
    previous
        .checked_add(1)
        .filter(|attempt| *attempt <= MAX_ATTEMPTS)
        .ok_or_else(|| terminal("teams source attempt budget is exhausted"))
}

fn slot_key(key: &str, attempt: u8, suffix: &str) -> String {
    format!("{key}/attempt/{attempt}/{suffix}")
}

fn terminal(message: &str) -> JobError {
    JobError::Terminal {
        message: message.to_string(),
    }
}
