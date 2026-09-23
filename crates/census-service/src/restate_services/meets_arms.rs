//! The meet-index stage's arms: the walk the stage runs for each planned source.
//!
//! Split from [`super::jobs`] the way the team-index arms are — the stage's own wrapper is one
//! concern and the walks it dispatches are another — and re-exported through `jobs` so a caller
//! names one module. [`meets_stage`] runs the state's own results index first, because that walk
//! reports the census's index counts, then every planned source this table arms.
//!
//! Every arm publishes meets from an index it can open by itself: the association's result archive
//! lists one meet per artifact and the timer's schedule lists one meet per competition row. A
//! source whose meet list needs a seed from another source is refused by the plan instead of being
//! run here, because a stage that invented its own seed would be a second opinion about what the
//! run covers.

use std::sync::Arc;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use restate_sdk::prelude::{HandlerError, Json, TerminalError};

use crate::census::{self, MeetCensus, MeetSourceRows};
use census_crawl::net::Fetcher;
use census_crawl::{Recorded, RecordedBatch, Recording};
use census_store::Store;
use serde::{Deserialize, Serialize};

use super::jobs::{
    adapter_context, append_observations, assert_some_stage_arms, collect_error, rows_written,
};
use super::{job_error, JobError, MAX_ROWS_PER_REQUEST};

/// The walks the meet-index stage can run, one per planned source.
///
/// The plan asks [`super::jurisdiction::DISPATCHED`] whether any stage sweeps a source; this table
/// and [`super::teams_arms::TEAMS_ARMS`] are what the chain answers that claim with. The slugs are
/// the *registry's* spellings — what a plan and a refusal carry — not the evidence tag each walk
/// stamps (`wiaa_results` publishes under its own slug, `wayzata` under `wayzata_schedule`), and
/// `jurisdiction::tests::the_arms_are_the_dispatched_slugs` holds the union of the two tables to
/// the dispatched list so a slug planned without an arm fails a test instead of failing a run.
pub(super) const MEETS_ARMS: &[(&str, MeetsArm)] = &[
    ("wiaa_results", MeetsArm::WiaaResults),
    ("wayzata", MeetsArm::Wayzata),
];

/// What the meet-index stage produced: the report its caller records, and the rows each routed
/// source's walk handed over instead of writing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct MeetsStageOutcome {
    /// The report: the index walk's counts, every source's row count, and the truncation flags.
    pub census: MeetCensus,
    /// One entry per source whose walk produced something to post, in the plan's order.
    pub recorded: Vec<RecordedSource>,
}

/// One planned source's walk output, recorded for the caller to post through its `Ingest` object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct RecordedSource {
    /// The planned slug, whose endpoint is `<slug>_<state>`.
    pub slug: String,
    /// The rows to post, and the journal entries to write once they are posted.
    pub recorded: Recorded,
}

/// One arm per walk: what the stage runs for a planned source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MeetsArm {
    /// The state association's own result archive: one artifact per meet, listed per season.
    WiaaResults,
    /// The timer's published schedules: one core meet per competition row, per sport and season.
    Wayzata,
}

/// The arm for one planned slug, or `None` when the meet-index stage has no walk for it.
///
/// `None` is not a source condition: the stage skips a unit another stage arms and refuses, through
/// [`assert_some_stage_arms`], a unit no stage arms at all.
pub(super) fn arm_for(slug: &str) -> Option<MeetsArm> {
    MEETS_ARMS
        .iter()
        .find(|(planned, _)| *planned == slug)
        .map(|(_, arm)| *arm)
}

/// The meet-index stage: enumerate one jurisdiction's published meets for one season year, one walk
/// per planned source, and report what each source wrote.
///
/// The state's own results index runs first and always: it is the walk whose counts the census
/// reports as its index fields, and its rows are the first entry in the per-source breakdown. Every
/// other planned source runs through [`MEETS_ARMS`]. Each walk journals what it read, so a stage
/// that runs again resumes instead of re-fetching, and the per-source rows are what tells an arm
/// that found nothing apart from an arm that never ran.
///
/// Every walk here is routed: the index's and each arm's rows are recorded, written to the store this
/// run holds, and handed to the caller to post through that source's `Ingest` object. The index walk
/// is the one source the stage does not look up in the plan — it *is* the plan's first entry
/// ([`census::SOURCE`]), and it runs whether or not anything else is armed.
pub(super) async fn meets_stage(
    store: Arc<Store>,
    fetcher: Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    year: u16,
    refresh: bool,
    at: String,
    sweepable: Vec<String>,
) -> Result<Json<MeetsStageOutcome>, HandlerError> {
    let index = Recording::new();
    let mut census = census::collect_state_meets(
        &fetcher,
        &store,
        jurisdiction,
        year,
        &at,
        refresh,
        Some(&index),
    )
    .await
    .map_err(collect_error)?;
    let season = season_of(year)?;
    let mut sources = vec![MeetSourceRows {
        slug: census::SOURCE.to_string(),
        rows: census.rows,
    }];
    let mut recorded = Vec::new();
    take_recorded(&store, census::SOURCE, index.drain(), &mut recorded)?;
    for slug in &sweepable {
        let Some(arm) = arm_for(slug) else {
            assert_some_stage_arms(slug)?;
            continue;
        };
        let recording = Recording::new();
        // The arm's sink: `Some` holds the rows for the caller to post, `None` writes them into the
        // store the walk holds. The walk itself is the same either way.
        let rows = match arm {
            MeetsArm::WiaaResults => {
                walk_wiaa_results(
                    &store,
                    &fetcher,
                    jurisdiction,
                    season,
                    refresh,
                    &at,
                    Some(&recording),
                )
                .await?
            }
            MeetsArm::Wayzata => {
                walk_wayzata(
                    &store,
                    &fetcher,
                    jurisdiction,
                    season,
                    refresh,
                    &at,
                    Some(&recording),
                )
                .await?
            }
        };
        take_recorded(&store, slug, recording.drain(), &mut recorded)?;
        sources.push(MeetSourceRows {
            slug: slug.clone(),
            rows,
        });
        census.rows = census.rows.saturating_add(rows);
    }
    census.sources = sources;
    Ok(Json(MeetsStageOutcome { census, recorded }))
}

/// Take one routed walk's recording: write its rows to the store this run holds, and keep them for
/// the caller to post.
///
/// Both destinations, deliberately. Written, because this run's own store is where the meets a later
/// stage selects are read from — the handoff is not allowed to move rows out of the table the run
/// already consumes them from. Recorded, because the caller posts them to the source's `Ingest`
/// object and only then writes the journal entries the walk produced: no unit is marked read before
/// the rows it produced are durable, in either place. A row that lands in both is one row by id, so
/// the handoff cannot double-write.
fn take_recorded(
    store: &Store,
    slug: &str,
    walked: Recorded,
    recorded: &mut Vec<RecordedSource>,
) -> Result<(), HandlerError> {
    if walked.is_empty() {
        return Ok(());
    }
    for batch in &walked.rows {
        append_recorded(store, batch)?;
    }
    recorded.push(RecordedSource {
        slug: slug.to_string(),
        recorded: walked,
    });
    Ok(())
}

/// The season the request asked for, as the walks carry it. A year the season constructor rejects
/// is a request fault rather than a source condition, so it is terminal.
fn season_of(year: u16) -> Result<SchoolYear, HandlerError> {
    let start_year = i16::try_from(year).map_err(|_| not_a_season(year))?;
    SchoolYear::new(start_year).ok_or_else(|| not_a_season(year))
}

/// The terminal error a year outside the season window earns.
fn not_a_season(year: u16) -> HandlerError {
    TerminalError::new(format!("season year {year} is not a school year")).into()
}

/// Write one recorded batch into the store this run holds, in request-sized chunks.
///
/// The batch the walk committed is the unit its journal entry covers, and it is written here for the
/// same reason the walk would have written it: a later stage selects meets from this table. The
/// per-request ceiling is the object route's, so a batch larger than one request is split rather than
/// refused — the rows are the walk's, not a caller's, and refusing them would fail a run over the
/// shape of a source page.
fn append_recorded(store: &Store, batch: &RecordedBatch) -> Result<(), HandlerError> {
    for chunk in batch.rows.chunks(MAX_ROWS_PER_REQUEST) {
        append_observations(store, batch.table, chunk)
            .map_err(|error| job_error(JobError::from(error)))?;
    }
    Ok(())
}

/// The association's result archive: its per-season listing is the meet index, so the season the
/// request asked for is the only seed this walk needs.
async fn walk_wiaa_results(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
    recording: Option<&Recording>,
) -> Result<usize, HandlerError> {
    let options = census_crawl::wiaa_results::Options {
        limit: None,
        refresh,
        observed_on: at.to_string(),
        seasons: vec![season.get()],
        states: vec![jurisdiction],
        school_names: Vec::new(),
    };
    let context = adapter_context(store, fetcher, season, refresh, at, recording);
    let report = census_crawl::wiaa_results::collect(&context, &options)
        .await
        .map_err(collect_error)?;
    rows_written(&report)
}

/// The timer's published schedules for the season the request asked for. It serves its own three
/// states from one set of pages, so the walk is the same one for each of them and the store merges
/// the rows it writes by id.
async fn walk_wayzata(
    store: &Arc<Store>,
    fetcher: &Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    refresh: bool,
    at: &str,
    recording: Option<&Recording>,
) -> Result<usize, HandlerError> {
    let options = census_crawl::wayzata::Options {
        years: vec![season.get()],
        limit: None,
        refresh,
        observed_on: Some(at.to_string()),
    };
    let context = adapter_context(store, fetcher, season, refresh, at, recording);
    let report = census_crawl::wayzata::collect(&context, &options)
        .await
        .map_err(collect_error)?;
    let _ = jurisdiction;
    rows_written(&report)
}
