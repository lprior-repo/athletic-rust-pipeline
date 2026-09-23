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
use census_crawl::net::Fetcher;

use super::JurisdictionCensus;
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

    /// Enumerate the jurisdiction's meets for one season year. The count and per-page journal are
    /// durable, so a re-invocation that reaches this stage again resumes at the first page this
    /// season has not already recorded.
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
        let Json(census) = ctx
            .run(move || {
                jobs::meets_stage(store, fetcher, jurisdiction, year, refresh, at, sweepable)
            })
            .retry_policy(jobs::no_run_retry())
            .await?;
        Ok(census)
    }
}
