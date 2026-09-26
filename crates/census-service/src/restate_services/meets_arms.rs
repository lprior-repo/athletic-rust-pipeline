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

mod walks;

use std::sync::Arc;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;
use restate_sdk::prelude::{HandlerError, Json, TerminalError};

use crate::census::{self, MeetCensus, MeetSourceRows};
use census_crawl::net::Fetcher;
use census_crawl::{Recorded, Recording};
use census_store::Store;
use serde::{Deserialize, Serialize};

use super::jobs::assert_some_stage_arms;
use walks::{walk_index, Walk};

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
/// Every walk here is routed: the index's and each arm's rows are recorded and handed to the
/// caller to post through that source's `Ingest` object, which is the single durability authority.
/// The index walk is the one source the stage does not look up in the plan — it *is* the plan's
/// first entry ([`census::SOURCE`]), and it runs whether or not anything else is armed.
pub(super) async fn meets_stage(
    store: Arc<Store>,
    fetcher: Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    year: u16,
    refresh: bool,
    at: String,
    sweepable: Vec<String>,
) -> Result<Json<MeetsStageOutcome>, HandlerError> {
    let season = season_of(year)?;
    let (mut census, index_recorded) =
        walk_index(&store, &fetcher, jurisdiction, year, refresh, &at).await?;
    let mut recorded = Vec::new();
    if let Some(entry) = index_recorded {
        recorded.push(entry);
    }
    let walk = Walk::new(&store, &fetcher, jurisdiction, season, refresh, &at);
    let mut sources = vec![MeetSourceRows {
        slug: census::SOURCE.to_string(),
        rows: census.rows,
    }];
    for slug in &sweepable {
        let Some(arm) = arm_for(slug) else {
            assert_some_stage_arms(slug)?;
            continue;
        };
        let recording = Recording::new();
        let rows = walk.armed(arm, &recording).await?;
        take_recorded(slug, recording.drain(), &mut recorded);
        sources.push(MeetSourceRows {
            slug: slug.clone(),
            rows,
        });
        census.rows = census.rows.saturating_add(rows);
    }
    census.sources = sources;
    Ok(Json(MeetsStageOutcome { census, recorded }))
}
/// Take one routed walk's recording: keep the rows for the caller to post through the source's
/// `Ingest` object.
///
/// The Ingest object is the single durability authority: it persists the recording, and the store
/// appends only through the `record` handler. The caller posts recorded rows to the source's
/// Ingest object and only then writes the journal entries the walk produced, so no unit is marked
/// read before the rows it produced are durable.
fn take_recorded(slug: &str, walked: Recorded, recorded: &mut Vec<RecordedSource>) {
    if walked.is_empty() {
        return;
    }
    recorded.push(RecordedSource {
        slug: slug.to_string(),
        recorded: walked,
    });
}

/// The season the request asked for, as the walks carry it. A year the season constructor rejects
/// is a request fault rather than a source condition, so it is terminal.
///
/// `pub(super)` because the results stage learns the year from the same request field and must
/// refuse it with the same sentence.
pub(super) fn season_of(year: u16) -> Result<SchoolYear, HandlerError> {
    let start_year = i16::try_from(year).map_err(|_| not_a_season(year))?;
    SchoolYear::new(start_year).ok_or_else(|| not_a_season(year))
}

/// The terminal error a year outside the season window earns.
pub(super) fn not_a_season(year: u16) -> HandlerError {
    TerminalError::new(format!("season year {year} is not a school year")).into()
}
