use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::restate_services::jurisdiction::team_source) struct Authorization {
    pub(super) version: u8,
    pub(in crate::restate_services::jurisdiction::team_source) original_digest: String,
    pub(super) original_attempt: u8,
    pub(super) request_digest: String,
    pub(super) observed_on: String,
    pub(super) origin: String,
}

pub(in crate::restate_services::jurisdiction::team_source) fn authorize(
    store: &Store,
    key: &str,
    request: &TeamsSourceRequest,
    expected: &str,
) -> Result<Authorization, JobError> {
    let snapshot = store.snapshot();
    let authorization = validate(&snapshot, key, Some(request))?;
    if authorization.original_digest != expected {
        return Err(terminal(
            "origin continuation original outcome digest mismatch",
        ));
    }
    if let Some(existing) = read::<Authorization>(&snapshot, &format!("{key}/authorization"))? {
        if existing != authorization {
            return Err(terminal(
                "origin continuation authorization contradicts original history",
            ));
        }
        history::read(&snapshot, key)?;
        return Ok(existing);
    }
    history::read(&snapshot, key)?;
    let mut batch = store.write_batch();
    batch.journal_done(PHASE, &format!("{key}/authorization"), &authorization)?;
    batch.commit()?;
    Ok(authorization)
}

pub(in crate::restate_services::jurisdiction::team_source) fn validate(
    snapshot: &StoreSnapshot<'_>,
    key: &str,
    request: Option<&TeamsSourceRequest>,
) -> Result<Authorization, JobError> {
    let identity: Identity = super::super::read(snapshot, &format!("{key}/identity"))?
        .ok_or_else(|| terminal("origin continuation original identity missing"))?;
    if let Some(request) = request {
        if request.source != "milesplit"
            || super::super::super::operation::phase(key, request)?
                != super::super::super::operation::Phase::Teams
            || super::super::request_digest(key, request)? != identity.request_digest
            || request.observed_on != identity.observed_on
        {
            return Err(terminal("origin continuation request identity mismatch"));
        }
    }
    let history = super::super::history::read(snapshot, key, &identity)?;
    let original = super::super::settlement::validated_settled(snapshot, key, &history, false)?
        .ok_or_else(|| terminal("origin continuation original is not settled"))?;
    let TeamsSourceOutcome::Terminal { message, progress } = &original else {
        return Err(terminal(
            "origin continuation requires a terminal ownership refusal",
        ));
    };
    if history.attempt == 0
        || history.attempt >= MAX_ATTEMPTS
        || progress
            .iter()
            .any(|step| matches!(step, TeamsAttemptProgress::Unknown { .. }))
        || !matches!(progress.last(), Some(TeamsAttemptProgress::Terminal { attempt, outcome: None, message: last }) if *attempt == history.attempt && last == message)
    {
        return Err(terminal(
            "origin continuation requires known original outcomes and remaining budget",
        ));
    }
    let origin = parse_origin(key, message)?;
    if let Some(request) = request {
        let expected = format!(
            "https://{}",
            census_crawl::milesplit::Site::for_jurisdiction(request.jurisdiction.jurisdiction)
                .host()
        );
        if origin != expected {
            return Err(terminal(
                "origin continuation refusal belongs to another source origin",
            ));
        }
    }
    Ok(Authorization {
        version: 1,
        original_digest: serialized_digest(&original)
            .map_err(|error| terminal(&error.to_string()))?,
        original_attempt: history.attempt,
        request_digest: identity.request_digest,
        observed_on: identity.observed_on,
        origin,
    })
}

fn parse_origin(key: &str, message: &str) -> Result<String, JobError> {
    if !key.ends_with("/teams/milesplit") {
        return Err(terminal(
            "origin continuation original source is not milesplit",
        ));
    }
    let (origin, holder) = message
        .strip_prefix("origin ")
        .and_then(|value| value.split_once(" is held by another census-service process (holder: "))
        .and_then(|(origin, holder)| holder.strip_suffix(')').map(|holder| (origin, holder)))
        .ok_or_else(|| {
            terminal("origin continuation original cause is not an ownership refusal")
        })?;
    let holder: serde_json::Value = serde_json::from_str(holder)
        .map_err(|_| terminal("origin continuation holder record is invalid"))?;
    if holder.get("origin").and_then(serde_json::Value::as_str) != Some(origin)
        || !holder
            .get("pid")
            .and_then(serde_json::Value::as_u64)
            .is_some_and(|pid| pid > 0 && u32::try_from(pid).is_ok())
    {
        return Err(terminal(
            "origin continuation holder origin or pid is invalid",
        ));
    }
    Ok(origin.into())
}
