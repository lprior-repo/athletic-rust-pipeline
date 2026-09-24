//! The stage runners: each stage's durable `run` wrapper, split from the state plumbing beside it.
//!
//! A stage has two halves. This module holds the first — the `*_stage` method that wraps its
//! [`jobs`] body in a durable `run` under the `no_run_retry` policy — and [`super::stages`] holds
//! the second, the `*_owed` method that calls it and records the outcome in the object's state
//! value. The seam is the one the module docs already draw; the file budget is only why it is a
//! module boundary rather than a section heading.
//!
//! The methods are `pub(super)` because the endpoint surface in `jurisdiction.rs` is the parent
//! module, and they reach the struct's fields the way [`super::stages`] does: both are children of
//! the module the fields are private to.

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
    /// Enumerate the jurisdiction's school and team universe, one walk per planned source. Every
    /// walk journals per unit, so a re-invocation that reaches this stage again resumes.
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
            .retry_policy(jobs::no_run_retry())
            .await?;
        Ok(outcome)
    }

    /// Walk every roster in the jurisdiction. The team index is read back from the journaled cache,
    /// so resuming costs a cache hit rather than a request.
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
            .retry_policy(jobs::no_run_retry())
            .await?;
        Ok(progress)
    }

    /// Pull the meets this run enumerated, one arm per planned result source.
    ///
    /// Nothing is routed from here: a results arm appends canonical observations through its own
    /// adapter, and the adapter's journal is the resume point (one entry per result set for
    /// `milesplit_results`, one per meet's request pair for `athleticnet`). The outcome is per source
    /// so that an arm which found no meets reads differently from an arm that never ran.
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
            .retry_policy(jobs::no_run_retry())
            .await?;
        Ok(outcome)
    }

    /// Enumerate the jurisdiction's meets for one season year. The count and per-page journal are
    /// durable, so a re-invocation that reaches this stage again resumes at the first page this
    /// season has not already recorded.
    ///
    /// Each planned source's walk runs with a recording as well as the store: its rows are written
    /// where the run reads them, and posted through that source's `Ingest` object — rows first, then
    /// the journal entries the walk produced — so the run's acquisition of the source is measurable
    /// on a durable object instead of being inferred from a report. The object calls are journaled
    /// like every other call this workflow makes: a replay re-reads their replies without appending
    /// twice.
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
        // The window is read before the walk, so a date the calendar cannot read refuses the stage
        // instead of leaving a run that acquired sources without a window to file them under.
        let window = ingest_post::window_of(&at)?;
        let Json(outcome) = ctx
            .run(move || {
                jobs::meets_stage(store, fetcher, jurisdiction, year, refresh, at, sweepable)
            })
            .retry_policy(jobs::no_run_retry())
            .await?;
        for source in &outcome.recorded {
            let endpoint = ingest_post::endpoint_of(&source.slug, jurisdiction);
            let posted = ingest_post::post(ctx, &endpoint, &window, &source.recorded.rows).await?;
            let store = Arc::clone(&self.store);
            let entries = source.recorded.journal.clone();
            let Json(journaled) = ctx
                .run(move || jobs::flush_journal(store, entries))
                .retry_policy(jobs::no_run_retry())
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
