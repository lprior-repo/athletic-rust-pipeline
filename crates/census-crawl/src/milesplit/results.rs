use super::map::ProviderSchools;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam,
    SourceObservation,
};
use census_domain::UsJurisdiction;
use std::collections::HashMap;

mod accumulator;
mod append;
mod budget;
mod frontier;
mod pages;
mod report;
mod run;
mod schools;

pub use pages::{
    is_results_page, read_meet_pages, ListedResultFile, MeetPage, MeetPages, MISMATCH_LIMIT,
};
use run::Run;

pub const RESULT_SET_PHASE: &str = "milesplit_result_sets_v5";
const ADAPTER: &str = "milesplit_results";
const APPLICATION_PHASE: &str = "milesplit_result_set_effects_v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultSetRequest {
    pub url: String,
    pub jurisdiction: UsJurisdiction,
}

#[derive(Debug, Clone, Default)]
pub struct ResultSetOptions {
    pub urls: Vec<ResultSetRequest>,
}

#[derive(Debug, Default)]
pub(super) struct Accumulator {
    pub(super) meets: HashMap<String, CanonicalMeet>,
    pub(super) events: HashMap<String, CanonicalEvent>,
    pub(super) teams: HashMap<String, CanonicalTeam>,
    pub(super) athletes: HashMap<String, CanonicalAthlete>,
    pub(super) performances: HashMap<String, CanonicalPerformance>,
    pub(super) observations: HashMap<String, SourceObservation>,
    pub(super) retained: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Default)]
pub(super) enum ResourceStop {
    #[default]
    Open,
    Unfinished(String),
}

#[derive(Debug, Default)]
pub(super) struct Stats {
    pub(super) result_sets: usize,
    pub(super) result_sets_resumed: usize,
    pub(super) result_sets_empty: usize,
    pub(super) failures: Vec<String>,
    pub(super) failure_count: usize,
    pub(super) unread: usize,
    pub(super) resource_stop: ResourceStop,
    pub(super) rows: usize,
    pub(super) rows_with_cohort: usize,
    pub(super) rows_without_cohort: usize,
    pub(super) rows_without_school: usize,
    pub(super) rows_without_name: usize,
    pub(super) rows_without_sport: usize,
    pub(super) events: usize,
    pub(super) skipped_lines: usize,
    pub(super) school_resolved: HashMap<&'static str, usize>,
    pub(super) unresolved: HashMap<String, usize>,
    pub(super) peak_window_bytes: usize,
    pub(super) peak_capture_bytes: usize,
}

#[tracing::instrument(skip(ctx, options))]
pub async fn collect(
    ctx: &AdapterContext<'_>,
    options: &ResultSetOptions,
) -> CrawlResult<AdapterReport> {
    if options.urls.is_empty() {
        let mut report = AdapterReport::new(ADAPTER, "result rows");
        report.note("no result-set URLs supplied; nothing was requested".to_string());
        return Ok(report);
    }
    let before = stats_of(ctx).await;
    let mut run = Run::new(schools::read(ctx)?);
    for (ordinal, request) in options.urls.iter().enumerate() {
        run.request(ctx, request, ordinal).await?;
    }
    run.release_owned();
    let after = stats_of(ctx).await;
    run.frontier.finish(
        ctx,
        options,
        report::finish(&run.stats, &run.counts, before, after)?,
    )
}

#[tracing::instrument(skip(ctx))]
async fn stats_of(ctx: &AdapterContext<'_>) -> (u64, u64) {
    let stats = ctx.fetcher.stats().await;
    (stats.physical_requests(), stats.cache_hits)
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct EntityCounts {
    pub(super) meets: usize,
    pub(super) events: usize,
    pub(super) teams: usize,
    pub(super) athletes: usize,
    pub(super) performances: usize,
    pub(super) unsupported_cohorts: usize,
}

impl EntityCounts {
    pub(super) fn add(&mut self, other: Self) {
        self.meets = self.meets.saturating_add(other.meets);
        self.events = self.events.saturating_add(other.events);
        self.teams = self.teams.saturating_add(other.teams);
        self.athletes = self.athletes.saturating_add(other.athletes);
        self.performances = self.performances.saturating_add(other.performances);
        self.unsupported_cohorts = self
            .unsupported_cohorts
            .saturating_add(other.unsupported_cohorts);
    }
}

fn journal_changed(
    ctx: &AdapterContext<'_>,
    page: &mut crate::recording::RowBatch<'_>,
    key: &str,
    payload: &serde_json::Value,
) -> CrawlResult<()> {
    if let Some(recording) = ctx.recording {
        if recording.inspect_journal(RESULT_SET_PHASE, key, |prior| {
            matching_receipt(prior, key, payload)
        })? {
            return Ok(());
        }
    }
    let prior = ctx.store.journal_payload(RESULT_SET_PHASE, key)?;
    if matching_receipt(prior.as_ref(), key, payload)? {
        Ok(())
    } else {
        page.journal_done(RESULT_SET_PHASE, key, payload)
    }
}

fn matching_receipt(
    prior: Option<&serde_json::Value>,
    key: &str,
    payload: &serde_json::Value,
) -> CrawlResult<bool> {
    match prior {
        Some(prior) if prior == payload => Ok(true),
        Some(_) => Err(CrawlError::Invariant {
            detail: format!("content-bound result projection receipt changed: {key}"),
        }),
        None => Ok(false),
    }
}

fn journal_contains(ctx: &AdapterContext<'_>, phase: &str, key: &str) -> CrawlResult<bool> {
    if let Some(recording) = ctx.recording {
        if recording.inspect_journal(phase, key, |prior| Ok(prior.is_some()))? {
            return Ok(true);
        }
    }
    Ok(ctx.store.journal_contains(phase, key)?)
}
