use super::super::{capture_admission::ResultsCaptureExecution, JurisdictionCensus};
use crate::restate_services::work_admission::WorkAdmission;
use crate::restate_services::{
    history_stage::HistoricalStageScope,
    jobs,
    results_arms::{plan_v2, ResultsStageOutcome},
    source_selection::SourceSelection,
};
use census_crawl::net::Fetcher;
use restate_sdk::prelude::*;
use std::sync::Arc;

pub(super) fn selected(capture: &ResultsCaptureExecution) -> bool {
    matches!(capture, ResultsCaptureExecution::FreshQualification { owner, .. }
        if owner.protocol() == census_domain::model::CaptureProtocolVersion::V2)
}

impl JurisdictionCensus {
    #[tracing::instrument(skip_all)]
    pub(super) async fn results_stage_v2(
        &self,
        ctx: &ObjectContext<'_>,
        fetcher: Arc<Fetcher>,
        scope: HistoricalStageScope,
        sources: SourceSelection,
        capture: ResultsCaptureExecution,
    ) -> Result<ResultsStageOutcome, HandlerError> {
        let ResultsCaptureExecution::FreshQualification { owner, .. } = &capture else {
            return Err(jobs::invariant(
                "v2 Results requires the admitted structured owner",
            ));
        };
        let owner = owner.as_ref().clone();
        let store = Arc::clone(&self.store);
        let planning_fetcher = Arc::clone(&fetcher);
        let region = self.region();
        let admission = self.jobs.admit_work(ctx.key()).await?;
        let handle = tokio::runtime::Handle::current();
        let plan = ctx
            .run(move || async move {
                worker(region, admission, move || {
                    handle.block_on(plan_v2::j0::prepare(
                        store,
                        planning_fetcher,
                        owner,
                        scope,
                        sources,
                    ))
                })
                .await
            })
            .name(plan_v2::PLAN_RUN)
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        let store = Arc::clone(&self.store);
        let region = self.region();
        let admission = self.jobs.admit_work(ctx.key()).await?;
        let handle = tokio::runtime::Handle::current();
        let Json(outcome) = ctx
            .run(move || async move {
                worker(region, admission, move || {
                    handle.block_on(plan_v2::execute::execute(
                        plan, store, fetcher, scope, sources, capture,
                    ))
                })
                .await
            })
            .name(plan_v2::EXECUTE_RUN)
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(outcome)
    }
}

async fn worker<T, F>(
    region: Arc<crate::spawn::Spawner>,
    admission: WorkAdmission,
    job: F,
) -> Result<T, HandlerError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, HandlerError> + Send + 'static,
{
    let outcome = region
        .dedicated(admission, job)
        .await
        .map_err(|error| jobs::invariant(&format!("capture worker admission: {error}")))?;
    match outcome {
        crate::outcome::Outcome::Ok(value) => Ok(value),
        crate::outcome::Outcome::Err(error) => Err(error),
        crate::outcome::Outcome::Panicked => {
            Err(jobs::invariant("capture results worker panicked"))
        }
        crate::outcome::Outcome::Cancelled => {
            Err(jobs::invariant("capture results worker cancelled"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excluded_milesplit_role_does_not_enter_the_v2_recovery_journal(
    ) -> Result<(), Box<dyn std::error::Error>> {
        check!(!selected(&ResultsCaptureExecution::UnadoptedSources));
        check!(!selected(&ResultsCaptureExecution::LegacyFresh));
        Ok(())
    }
}
