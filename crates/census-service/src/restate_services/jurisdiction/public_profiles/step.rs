use super::*;
use crate::census::profiles::{self, CaptureOutcome, ProfileSource, Selection, Target};
use crate::restate_services::{blocking, jobs};
use census_crawl::net::Fetcher;
use census_domain::model::GradYear;
use census_store::CaptureWriterV1;
use std::sync::Arc;

pub(super) struct CaptureStep<'a> {
    pub(super) request: &'a JurisdictionRequest,
    pub(super) writer: &'a Arc<CaptureWriterV1>,
    pub(super) fetcher: &'a Arc<Fetcher>,
    pub(super) target: &'a Target,
}

pub(super) struct Batch<'a> {
    pub(super) request: &'a JurisdictionRequest,
    pub(super) writer: &'a Arc<CaptureWriterV1>,
    pub(super) fetcher: &'a Arc<Fetcher>,
    pub(super) cohort: GradYear,
    pub(super) rosters_terminal: bool,
}

impl JurisdictionCensus {
    pub(super) async fn run_profile_batch(
        &self,
        ctx: &ObjectContext<'_>,
        batch: Batch<'_>,
        steps: u16,
        progress: &mut Progress,
    ) -> Result<(), HandlerError> {
        for _ in 0..steps {
            match self
                .next_profile(ctx, batch.request, batch.cohort, progress)
                .await?
            {
                Selection::Profile(target) => {
                    let outcome = self
                        .capture_profile(
                            ctx,
                            CaptureStep {
                                request: batch.request,
                                writer: batch.writer,
                                fetcher: batch.fetcher,
                                target: &target,
                            },
                        )
                        .await?;
                    progress
                        .record(target, outcome)
                        .map_err(|error| job_error(error.into()))?;
                }
                Selection::Advance { next, gaps } => {
                    progress
                        .advance(next, gaps)
                        .map_err(|error| job_error(error.into()))?;
                }
                Selection::End => progress.finish_scan(batch.rosters_terminal),
            }
            ctx.set(KEY, Json(progress.clone()));
            if progress.stopped || progress.exhausted || progress.awaiting_rosters {
                break;
            }
        }
        Ok(())
    }

    pub(super) async fn next_profile(
        &self,
        ctx: &ObjectContext<'_>,
        request: &JurisdictionRequest,
        cohort: GradYear,
        progress: &Progress,
    ) -> Result<Selection, HandlerError> {
        let store = Arc::clone(&self.store);
        let region = self.region();
        let cursor = progress.cursor.clone();
        let site = request.jurisdiction;
        let Json(selection) = ctx
            .run(move || async move {
                let permit = self.jobs.permit_control().await?;
                blocking(region, move || {
                    let _permit = permit;
                    profiles::select(&store, site, cohort, cursor.as_ref())
                        .map(Json)
                        .map_err(jobs::collect_error)
                })
                .await
                .map_err(job_error)
            })
            .name("public_profiles:select")
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(selection)
    }

    pub(super) async fn capture_profile(
        &self,
        ctx: &ObjectContext<'_>,
        step: CaptureStep<'_>,
    ) -> Result<CaptureOutcome, HandlerError> {
        for attempt in 1..=3 {
            let store = Arc::clone(&self.store);
            let writer = Arc::clone(step.writer);
            let fetcher = Arc::clone(step.fetcher);
            let target = step.target.clone();
            let site = step.request.jurisdiction;
            let refresh = step.request.refresh;
            let region = self.region();
            let handle = tokio::runtime::Handle::current();
            let Json(outcome) = ctx
                .run(move || async move {
                    let permit = self.jobs.permit().await?;
                    blocking(region, move || {
                        let _permit = permit;
                        handle
                            .block_on(
                                ProfileSource::new(&store, &writer, &fetcher, site, refresh)
                                    .capture(&target),
                            )
                            .map(Json)
                            .map_err(jobs::collect_error)
                    })
                    .await
                    .map_err(job_error)
                })
                .name(format!("public_profiles:capture:{attempt}"))
                .retry_policy(RunRetryPolicy::new().max_attempts(1))
                .await?;
            if attempt == 3
                || !matches!(
                    outcome,
                    CaptureOutcome::SourceFailure {
                        retryable: true,
                        ..
                    }
                )
            {
                return Ok(outcome);
            }
        }
        Err(TerminalError::new("profile bounded acquisition has no outcome").into())
    }
}
