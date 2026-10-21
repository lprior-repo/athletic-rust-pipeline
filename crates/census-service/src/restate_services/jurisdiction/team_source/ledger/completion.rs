use super::settlement::{classify, settle, validated_settled};
use super::{
    attempt, history, read, slot_key, terminal, Attempt, AttemptOutcome, Completion, Identity,
    MAX_ATTEMPTS, PHASE,
};
use crate::restate_services::support::JobError;
use census_store::Store;

#[cfg(test)]
mod tests;

pub(in crate::restate_services::jurisdiction::team_source) fn finish(
    store: &Store,
    key: &str,
    attempt: u8,
    result: Attempt,
    conditions: &[census_domain::model::SourceAccessCondition],
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
    let (outcome, resource) = result.into_outcome(&identity.observed_on)?;
    let recorded = history
        .progress
        .last_mut()
        .ok_or_else(|| terminal("teams source reservation has no progress slot"))?;
    *recorded = attempt::progress(attempt, &outcome);
    let completion = classify(attempt, history.uncertain, &outcome, &history.progress);
    let persisted = (|| -> census_store::StoreResult<()> {
        let mut batch = store.write_batch();
        crate::census::access::stage(&mut batch, conditions)?;
        batch.journal_done(PHASE, &slot_key(key, attempt, "outcome"), &outcome)?;
        if let Completion::Settled(outcome) = &completion {
            batch.journal_done(PHASE, &format!("{key}/settled"), outcome)?;
        }
        #[cfg(test)]
        tests::refuse_persistence()?;
        batch.commit()?;
        Ok(())
    })();
    if let Err(secondary) = persisted {
        return Err(persistence_failure(&outcome, resource, secondary));
    }
    Ok(completion)
}

pub(super) fn persistence_failure(
    outcome: &AttemptOutcome,
    resource: Option<census_report::report::ExportResourceError>,
    secondary: census_store::StoreError,
) -> JobError {
    let primary = match outcome {
        AttemptOutcome::Completed { .. } => return secondary.into(),
        AttemptOutcome::Transient { message, .. } | AttemptOutcome::Terminal { message, .. } => {
            message
        }
    };
    let message = crate::census::access::combined_failure(primary, &secondary).to_string();
    if let Some(source) = resource {
        return JobError::Resource { source, message };
    }
    match outcome {
        AttemptOutcome::Transient { .. } => JobError::Transient { message },
        AttemptOutcome::Terminal { .. } => JobError::Terminal { message },
        AttemptOutcome::Completed { .. } => secondary.into(),
    }
}
