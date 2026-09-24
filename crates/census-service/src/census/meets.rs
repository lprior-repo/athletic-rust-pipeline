//! The meet census: one jurisdiction's published meets, enumerated from its state results index.
//!
//! # Request cost, measured
//!
//! The results index serves 50 meet rows per response (`samples/results-oh.html`, captured
//! 2026-09-22: exactly 50 `data-meet-id` attributes in one 187 KB body), and the page publishes its
//! own next page (`rel="next"`). A season's census is therefore `ceil(meets / 50)` requests, which is
//! why the meet census enumerates here instead of by walking meet pages: a 3,000-meet season costs
//! sixty responses, and every one of them is an ordinary HTML page on an allowed path.
//!
//! # What a page leaves behind
//!
//! One [`SourceMeetRef`] per row, keyed `milesplit:{MeetID}` (§31), written to the `source_meets`
//! table. The rows are what stage D reads: a results pull needs the provider's own meet id, and this
//! is the only place the census learns it without an operator list.
//!
//! Each page is journaled as it lands, so a re-invocation resumes at the first page whose body is not
//! already journaled, and a re-run costs cache hits rather than requests. The loop is bounded by
//! [`MAX_PAGES_PER_SEASON`] as well as by the pager: a site that answered `rel="next"` forever would
//! otherwise be an unbounded crawl, and §37 forbids that.
//!
//! The page walk itself lives in `walk`: it owns the journal reads, the per-page counters and the row
//! writes, and this module folds what it returns into a census and writes the table.

use census_crawl::milesplit::{MeetRef, Season, Site};
use census_crawl::net::Fetcher;
use census_crawl::recording::RowSink;
use census_crawl::{CrawlResult, Recording};
use census_domain::model::SourceMeetRef;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tracing::info;

mod walk;

use walk::{walk_season, SeasonReader, SeasonWalk};
/// Which seasons the meet selector should include.
///
/// A season-specific scope filters `source_meets` rows by their stored year before
/// applying the jurisdiction filter and limit. The all-seasons variant selects
/// every row regardless of year — useful for backfills or cross-season reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeasonScope {
    /// Only meets whose stored year matches.
    Year(u16),
    /// Every stored season, regardless of year.
    All,
}


/// The source namespace every row this module writes is keyed under.
pub const SOURCE: &str = "milesplit";

/// The most pages one season may have before the census stops reading it: four hundred pages of
/// fifty meets, so twenty thousand meets in one state's season.
///
/// A bound, not a target — the pager's own `rel="next"` is what normally ends the walk. The first
/// value tried here was sixty, and the Ohio walk of 2026-09-22 hit it with pages still owed
/// (`truncated=1`), so it was raised rather than kept as a silently incomplete census.
pub const MAX_PAGES_PER_SEASON: u32 = 400;

/// What one jurisdiction's meet census read.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeetCensus {
    /// Index pages this walk read, journaled or not.
    pub pages: usize,
    /// Of those, the ones this run took from the network.
    pub fetched: usize,
    /// Meets the pages published, before de-duplication.
    pub seen: usize,
    /// Rows appended to `source_meets` after de-duplication, across every source the stage ran: the
    /// state's own results index plus each armed source's walk. The index fields above describe the
    /// index walk alone; this is the whole stage's output.
    pub rows: usize,
    /// Seasons the run read.
    pub seasons: usize,
    /// Seasons whose walk stopped at `MAX_PAGES_PER_SEASON` while the pager still offered a next
    /// page. Non-zero means an unbounded crawl was cut short, so the census is missing meets rather
    /// than complete — the one condition this stage must never report silently.
    pub truncated: usize,
    /// Seasons that ended because the pager served one page twice, which is how this site's index
    /// terminates: the last page keeps advertising a next one. A season that ends this way is
    /// complete for the site's own listing.
    pub repeated: usize,
    /// What each source this stage ran wrote, in the plan's order: the state's own index first, then
    /// every planned source whose arm publishes meets of its own.
    ///
    /// Defaulted on read: a state recorded before this field existed carries the counts without the
    /// breakdown, and refusing to read it would turn a schema addition into a lost census.
    #[serde(default)]
    pub sources: Vec<MeetSourceRows>,
}

/// One source's share of a meet census: the slug the plan named and the `source_meets` rows that
/// source's walk appended.
///
/// The stage reports per source rather than in aggregate because the count is the only record that
/// a source was walked at all: an arm that found nothing and an arm that never ran would otherwise
/// both leave the same trace.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeetSourceRows {
    /// The registry slug, the spelling a plan and a refusal carry.
    pub slug: String,
    /// Rows this source appended to `source_meets`.
    pub rows: usize,
}

/// Enumerate one jurisdiction's meets for one season year, and write them as `source_meets` rows.
///
/// `observed_on` is the ISO day stamped into every row, supplied by the caller's clock so the domain
/// never reads a clock of its own.
impl MeetCensus {
    /// Add one season's walk to the census.
    fn fold(&mut self, walk: &SeasonWalk) {
        self.pages = self.pages.saturating_add(walk.pages);
        self.fetched = self.fetched.saturating_add(walk.fetched);
        self.seen = self.seen.saturating_add(walk.seen);
        self.repeated = self.repeated.saturating_add(walk.repeated);
        self.truncated = self.truncated.saturating_add(walk.truncated);
    }
}

pub async fn collect_state_meets(
    fetcher: &Fetcher,
    store: &Store,
    jurisdiction: UsJurisdiction,
    year: u16,
    observed_on: &str,
    refresh: bool,
    recording: Option<&Recording>,
) -> CrawlResult<MeetCensus> {
    let site = Site::for_jurisdiction(jurisdiction);
    let mut census = MeetCensus::default();
    let mut rows: BTreeMap<String, SourceMeetRef> = BTreeMap::new();
    // The sink decides who writes: the store this walk holds on a run that owns its rows, or the
    // caller's recording on a run whose acquisition is posted to an `Ingest` object. The walk itself
    // is the same either way.
    let sink = match recording {
        Some(recording) => RowSink::Record(recording),
        None => RowSink::Store(store),
    };
    for season in Season::ALL {
        let reader = SeasonReader {
            site,
            jurisdiction,
            season,
            year,
            observed_on,
            refresh,
        };
        census.seasons = census.seasons.saturating_add(1);
        let walk = walk_season(fetcher, store, &reader, &mut rows, &sink).await?;
        census.fold(&walk);
    }
    let rows: Vec<SourceMeetRef> = rows.into_values().collect();
    // Appended, not replaced: a row is an observation of a page, and the store is what keeps an
    // earlier observation when a later one disagrees. Same-id rows merge, so re-running a census
    // never grows the table past one row per meet. A routed run hands the rows to its recording
    // instead — the store write then belongs to its caller, which posts them before the markers.
    let mut batch = sink.write_batch();
    batch.append_many(Table::SourceMeets, &rows)?;
    batch.commit()?;
    census.rows = rows.len();
    info!(
        state = jurisdiction.code(),
        pages = census.pages,
        seen = census.seen,
        rows = census.rows,
        "meet census collected"
    );
    Ok(census)
}

/// The meets a results run should read: the stored rows for these states, in `(state, meet id)`
/// order, at most `limit` of them per state.
///
/// Pure, and deliberately so: the caller owns the store scan, and this decides only what the run
/// takes. Order comes from the provider's own id, so two runs over the same store read the same
/// meets in the same sequence. The `scope` parameter selects by stored year before any
/// filtering or limit is applied, so per-state limit counts are per-season, not cross-season.
pub fn select_meets(
    rows: Vec<SourceMeetRef>,
    states: &[UsJurisdiction],
    scope: SeasonScope,
    limit: Option<usize>,
) -> Vec<SourceMeetRef> {
    let rows = filter_by_season(rows, scope);
    let mut selected: Vec<SourceMeetRef> = rows
        .into_iter()
        .filter(|row| states.contains(&row.jurisdiction))
        .collect();
    selected.sort_by(|left, right| {
        (left.jurisdiction, left.source_meet_id.as_str())
            .cmp(&(right.jurisdiction, right.source_meet_id.as_str()))
    });
    let Some(limit) = limit else {
        return selected;
    };
    let mut taken: BTreeMap<UsJurisdiction, usize> = BTreeMap::new();
    selected.retain(|row| {
        let count = taken.entry(row.jurisdiction).or_insert(0);
        if *count >= limit {
            return false;
        }
        *count = count.saturating_add(1);
        true
    });
    selected
}
/// Filter to only the requested season: a row whose stored year matches.
///
/// Called before jurisdiction filtering and limit application so the chunking
/// limit is applied to the correct season's meets, not to a cross-season merge.
fn filter_by_season(rows: Vec<SourceMeetRef>, scope: SeasonScope) -> Vec<SourceMeetRef> {
    match scope {
        SeasonScope::All => rows,
        SeasonScope::Year(year) => rows.into_iter().filter(|row| row.year == year).collect(),
    }
}

/// Whether an index page repeats the page before it, which is this site's end-of-index signal.
///
/// A first page (`None`) never repeats, and an empty page only repeats an empty one: a meet index
/// that publishes nothing at all is a state, not a wrap.
fn repeats_previous(previous: Option<&[String]>, current: &[String]) -> bool {
    previous.is_some_and(|previous| previous == current)
}

/// The journal phase of one jurisdiction-season's results index.
pub fn meets_phase(jurisdiction: UsJurisdiction, season: Season, year: u16) -> String {
    format!(
        "milesplit_meet_index_{}_{}_{year}_v1",
        jurisdiction.code().to_ascii_lowercase(),
        season.code()
    )
}

/// One enumerated row as the store's own record.
fn source_meet_row(
    meet: &MeetRef,
    jurisdiction: UsJurisdiction,
    season: Season,
    year: u16,
    observed_on: &str,
) -> SourceMeetRef {
    SourceMeetRef {
        id: SourceMeetRef::row_id(SOURCE, &meet.meet_id),
        source: SOURCE.to_string(),
        source_meet_id: meet.meet_id.clone(),
        jurisdiction,
        season: season.code().to_string(),
        year,
        name: meet.name.clone(),
        date: meet.date.clone(),
        venue: meet.venue.clone(),
        results_url: meet.results_url.clone(),
        observed_on: observed_on.to_string(),
    }
}
#[cfg(test)]
#[path = "meets/tests.rs"]
mod tests;
