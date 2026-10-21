use super::*;

pub(super) fn read(snapshot: &StoreSnapshot<'_>, key: &str) -> Result<Option<State>, JobError> {
    let Some(authorization) =
        super::read::<Authorization>(snapshot, &format!("{key}/authorization"))?
    else {
        refuse_orphans(snapshot, key)?;
        return Ok(None);
    };
    let mut state = initialize_original(snapshot, key, authorization)?;
    read_reservations(snapshot, key, &mut state)?;
    verify_settlement(snapshot, key, &mut state)?;
    Ok(Some(state))
}

fn initialize_original(
    snapshot: &StoreSnapshot<'_>,
    key: &str,
    authorization: Authorization,
) -> Result<State, JobError> {
    if authorization != validate(snapshot, key, None)? {
        return Err(terminal(
            "origin continuation authority no longer matches original history",
        ));
    }
    let identity = Identity {
        request_digest: authorization.request_digest.clone(),
        observed_on: authorization.observed_on.clone(),
    };
    let original = super::super::history::read(snapshot, key, &identity)?;
    Ok(State {
        attempt: authorization.original_attempt,
        authorization,
        progress: original.progress,
        missing: false,
        uncertain: false,
        settled: None,
        transient: None,
    })
}

fn read_reservations(
    snapshot: &StoreSnapshot<'_>,
    key: &str,
    state: &mut State,
) -> Result<(), JobError> {
    let mut gap = false;
    for attempt in 1..=MAX_ATTEMPTS {
        let reservation: Option<Reservation> =
            super::read(snapshot, &slot_key(key, attempt, "reserved"))?;
        let outcome: Option<AttemptOutcome> =
            super::read(snapshot, &slot_key(key, attempt, "outcome"))?;
        let Some(reservation) = reservation else {
            if outcome.is_some() {
                return Err(terminal("origin continuation outcome has no reservation"));
            }
            gap |= attempt > state.authorization.original_attempt;
            continue;
        };
        validate_reservation(state, attempt, &reservation, gap)?;
        fold_reserved_attempt(state, attempt, outcome)?;
    }
    Ok(())
}

fn validate_reservation(
    state: &State,
    attempt: u8,
    reservation: &Reservation,
    gap: bool,
) -> Result<(), JobError> {
    if gap
        || state.settled.is_some()
        || super::super::next_attempt(state.attempt)? != attempt
        || reservation.attempt != attempt
        || reservation.observed_on != state.authorization.observed_on
    {
        return Err(terminal(
            "origin continuation reservation is noncontiguous or contradicts history",
        ));
    }
    Ok(())
}

fn fold_reserved_attempt(
    state: &mut State,
    attempt: u8,
    outcome: Option<AttemptOutcome>,
) -> Result<(), JobError> {
    state.uncertain |= state.missing;
    state.attempt = attempt;
    state.missing = outcome.is_none();
    state.transient = None;
    state.progress.push(match &outcome {
        Some(value) => attempt::progress(attempt, value),
        None => TeamsAttemptProgress::Unknown { attempt },
    });
    if let Some(outcome) = outcome {
        attempt::validate(&outcome, &state.authorization.observed_on)?;
        match settlement::classify(attempt, state.uncertain, &outcome, &state.progress) {
            Completion::Settled(outcome) => state.settled = Some(outcome),
            Completion::Retry(JobError::Transient { message }) => state.transient = Some(message),
            Completion::Retry(_) => {
                return Err(terminal("origin continuation retry classification invalid"))
            }
        }
    }
    Ok(())
}

fn verify_settlement(
    snapshot: &StoreSnapshot<'_>,
    key: &str,
    state: &mut State,
) -> Result<(), JobError> {
    let persisted: Option<TeamsSourceOutcome> = super::read(snapshot, &format!("{key}/settled"))?;
    if let Some(persisted) = persisted {
        let Some(expected) = final_outcome(state) else {
            return Err(terminal(
                "origin continuation settlement has no supporting history",
            ));
        };
        if serialized_digest(&persisted).map_err(|error| terminal(&error.to_string()))?
            != serialized_digest(&expected).map_err(|error| terminal(&error.to_string()))?
        {
            return Err(terminal(
                "origin continuation settlement contradicts its history",
            ));
        }
        state.settled = Some(persisted);
    }
    Ok(())
}

fn refuse_orphans(snapshot: &StoreSnapshot<'_>, key: &str) -> Result<(), JobError> {
    for attempt in 1..=MAX_ATTEMPTS {
        if super::read::<serde_json::Value>(snapshot, &slot_key(key, attempt, "reserved"))?
            .is_some()
            || super::read::<serde_json::Value>(snapshot, &slot_key(key, attempt, "outcome"))?
                .is_some()
        {
            return Err(terminal("origin continuation slots have no authorization"));
        }
    }
    if super::read::<serde_json::Value>(snapshot, &format!("{key}/settled"))?.is_some() {
        return Err(terminal(
            "origin continuation settlement has no authorization",
        ));
    }
    Ok(())
}
