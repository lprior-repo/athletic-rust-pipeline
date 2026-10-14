use super::JurisdictionCensus;
use crate::restate_services::{
    history_stage::HistoricalStageScope, jobs, results_arms::ResultsStageOutcome,
    source_selection::SourceSelection,
};
use census_crawl::net::Fetcher;
use restate_sdk::prelude::*;
use std::sync::Arc;

enum ResultsWork {
    Legacy,
    Admitted(crate::restate_services::work_admission::WorkAdmission),
}

impl JurisdictionCensus {
    pub(super) async fn results_stage(
        &self,
        ctx: &ObjectContext<'_>,
        fetcher: Arc<Fetcher>,
        scope: HistoricalStageScope,
        sources: SourceSelection,
        capture: super::capture_admission::ResultsCaptureExecution,
    ) -> Result<ResultsStageOutcome, HandlerError> {
        use super::capture_admission::ResultsCaptureExecution;
        let store = Arc::clone(&self.store);
        let region = self.region();
        let work = match &capture {
            ResultsCaptureExecution::LegacyFresh => ResultsWork::Legacy,
            ResultsCaptureExecution::UnadoptedSources
            | ResultsCaptureExecution::FreshQualificationV1 { .. } => {
                ResultsWork::Admitted(self.jobs.admit_work(ctx.key()).await?)
            }
        };
        let handle = tokio::runtime::Handle::current();
        let Json(outcome) = ctx
            .run(move || async move {
                match work {
                    ResultsWork::Legacy => {
                        jobs::results_stage(store, fetcher, scope, sources, capture).await
                    }
                    ResultsWork::Admitted(admission) => {
                        let outcome = region
                            .dedicated(admission, move || {
                                handle.block_on(jobs::results_stage(
                                    store, fetcher, scope, sources, capture,
                                ))
                            })
                            .await
                            .map_err(|error| {
                                jobs::invariant(&format!("capture worker admission: {error}"))
                            })?;
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
                }
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(outcome)
    }
}

#[cfg(test)]
mod replay_tests;
