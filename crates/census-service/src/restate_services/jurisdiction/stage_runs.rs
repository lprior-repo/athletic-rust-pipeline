use std::sync::Arc;

use restate_sdk::prelude::*;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;

use crate::census::{CollectOptions, MeetCensus, StateProgress};
use crate::restate_services::results_arms::ResultsStageOutcome;
use census_crawl::net::Fetcher;

use super::JurisdictionCensus;
use crate::restate_services::ingest_post;
use crate::restate_services::jobs;
use crate::restate_services::wire::StageOutcome;

impl JurisdictionCensus {
    pub(super) async fn teams_stage(
        &self,
        ctx: &ObjectContext<'_>,
        fetcher: Arc<Fetcher>,
        jurisdiction: UsJurisdiction,
        season: SchoolYear,
        refresh: bool,
        sweepable: Vec<String>,
    ) -> Result<StageOutcome, HandlerError> {
        let store = Arc::clone(&self.store);
        let at = super::super::journaled_today(ctx, &self.clock).await?;
        let Json(outcome) = ctx
            .run(move || {
                jobs::teams_stage(store, fetcher, jurisdiction, season, refresh, at, sweepable)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(outcome)
    }

    pub(super) async fn rosters_stage(
        &self,
        ctx: &ObjectContext<'_>,
        fetcher: Arc<Fetcher>,
        options: CollectOptions,
        jurisdiction: UsJurisdiction,
    ) -> Result<StateProgress, HandlerError> {
        let store = Arc::clone(&self.store);
        let Json(progress) = ctx
            .run(move || jobs::rosters_stage(store, fetcher, options, jurisdiction))
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(progress)
    }

    pub(super) async fn results_stage(
        &self,
        ctx: &ObjectContext<'_>,
        fetcher: Arc<Fetcher>,
        jurisdiction: UsJurisdiction,
        year: u16,
        refresh: bool,
        sweepable: Vec<String>,
    ) -> Result<ResultsStageOutcome, HandlerError> {
        let store = Arc::clone(&self.store);
        let at = super::super::journaled_today(ctx, &self.clock).await?;
        let Json(outcome) = ctx
            .run(move || {
                jobs::results_stage(store, fetcher, jurisdiction, year, refresh, at, sweepable)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        Ok(outcome)
    }

    pub(super) async fn meets_stage(
        &self,
        ctx: &ObjectContext<'_>,
        fetcher: Arc<Fetcher>,
        jurisdiction: UsJurisdiction,
        year: u16,
        refresh: bool,
        sweepable: Vec<String>,
    ) -> Result<MeetCensus, HandlerError> {
        let store = Arc::clone(&self.store);
        let at = super::super::journaled_today(ctx, &self.clock).await?;
        let window = ingest_post::window_of(&at)?;
        let Json(outcome) = ctx
            .run(move || {
                jobs::meets_stage(store, fetcher, jurisdiction, year, refresh, at, sweepable)
            })
            .retry_policy(RunRetryPolicy::new().max_attempts(1))
            .await?;
        for source in &outcome.recorded {
            let endpoint = ingest_post::endpoint_of(&source.slug, jurisdiction);
            let posted = ingest_post::post(ctx, &endpoint, &window, &source.recorded.rows).await?;
            let store = Arc::clone(&self.store);
            let entries = source.recorded.journal.clone();
            let Json(journaled) = ctx
                .run(move || jobs::flush_journal(store, entries))
                .retry_policy(RunRetryPolicy::new().max_attempts(1))
                .await?;
            tracing::info!(
                endpoint = endpoint.as_str(),
                window = window.as_str(),
                posted,
                journaled,
                "routed a source's acquisition through its ingest object"
            );
        }
        Ok(outcome.census)
    }
}
