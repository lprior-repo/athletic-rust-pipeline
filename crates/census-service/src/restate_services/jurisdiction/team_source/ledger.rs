use census_domain::model::serialized_digest;
use census_store::{Store, StoreSnapshot};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::outcome::AttemptState;
use crate::restate_services::support::JobError;
use crate::restate_services::wire::{
    StageOutcome, TeamsAttemptProgress, TeamsSourceInspection, TeamsSourceOutcome,
    TeamsSourceRequest,
};

mod admission;
mod attempt;
mod completion;
mod history;
mod registration;
mod settlement;
#[cfg(test)]
mod stable_slot_tests;
pub(super) use admission::{baseline, begin_at, AdmissionReplay};
pub(super) use attempt::Attempt;
pub(super) use completion::finish;
pub(super) use registration::register;
use settlement::validated_settled;

pub(super) const PHASE: &str = "teams_source_attempts_v1";
pub(super) const MAX_ATTEMPTS: u8 = 3;

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

pub(crate) fn durable_attempts(store: &Store) -> Result<AttemptState, JobError> {
    let snapshot = store.snapshot();
    let keys = store.journal_keys(PHASE)?;
    let mut operations: Vec<String> = keys
        .iter()
        .filter_map(|key| key.strip_suffix("/identity").map(str::to_string))
        .collect();
    operations.sort_unstable();
    let mut state = AttemptState {
        ceiling: MAX_ATTEMPTS,
        ..AttemptState::default()
    };
    for key in operations {
        let identity: Identity = read(&snapshot, &format!("{key}/identity"))?
            .ok_or_else(|| terminal("teams source attempt state has no identity"))?;
        let history = history::read(&snapshot, &key, &identity)?;
        state.sources = state.sources.saturating_add(1);
        state.reserved = state.reserved.saturating_add(u64::from(history.attempt));
        for progress in &history.progress {
            if !matches!(progress, TeamsAttemptProgress::Unknown { .. }) {
                state.recorded = state.recorded.saturating_add(1);
            }
        }
        if validated_settled(&snapshot, &key, &history, false)?.is_some() {
            state.settled = state.settled.saturating_add(1);
        }
    }
    state.unknown = state.reserved.saturating_sub(state.recorded);
    Ok(state)
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
    let phase = super::operation::phase(key, request)?;
    let subject = (
        jurisdiction.jurisdiction,
        jurisdiction.season,
        jurisdiction.revision,
        &request.source,
    );
    let digest = match phase {
        super::operation::Phase::Teams => serialized_digest(&subject),
        super::operation::Phase::SchoolContacts => serialized_digest(&(subject, "school_contacts")),
    };
    digest.map_err(|error| terminal(&format!("digesting source phase identity: {error}")))
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
