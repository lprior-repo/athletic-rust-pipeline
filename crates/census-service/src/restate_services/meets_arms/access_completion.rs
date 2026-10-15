use super::{diagnostics, MeetsStageOutcome, OUTCOME_PHASE};
use crate::restate_services::{job_error, jobs, JobError};
use census_crawl::net::Fetcher;
use census_domain::model::SourceAccessCondition;
use census_store::{Store, StoreResult};
use restate_sdk::prelude::HandlerError;

#[tracing::instrument(skip_all, fields(fragment = %outcome.identity))]
pub(super) async fn retain_and_complete(
    store: &Store,
    fetcher: &Fetcher,
    outcome: &MeetsStageOutcome,
) -> Result<(), HandlerError> {
    let conditions = fetcher.access_conditions().await.map_err(|error| {
        job_error(retention_failure(
            &outcome.identity,
            jobs::collect_error(error),
        ))
    })?;
    persist(store, outcome, &conditions)
        .map_err(|error| job_error(retention_failure(&outcome.identity, error.into())))
}

fn persist(
    store: &Store,
    outcome: &MeetsStageOutcome,
    conditions: &[SourceAccessCondition],
) -> StoreResult<()> {
    let mut batch = store.write_batch();
    crate::census::access::stage(&mut batch, conditions)?;
    batch.journal_done(OUTCOME_PHASE, &outcome.identity, outcome)?;
    batch.commit()
}

fn retention_failure(identity: &str, failure: JobError) -> JobError {
    let message = crate::census::access::combined_failure(
        &format_args!(
            "meet fragment diagnostics={}/{identity}",
            diagnostics::PHASE
        ),
        &failure,
    )
    .to_string();
    match failure {
        JobError::Resource { source, .. } => JobError::Resource { source, message },
        JobError::Transient { .. } => JobError::Transient { message },
        JobError::Terminal { .. } => JobError::Terminal { message },
    }
}

#[cfg(test)]
mod tests;
