//! One season's page walk: the loop, the page read, and the two conditions that end it.
//!
//! Split out of `meets.rs` so that file stays inside the one-page budget. The walk owns the
//! journal reads, the per-page counters and the row writes; `meets.rs` folds what it returns
//! into the census and writes the table.

use census_crawl::milesplit::{self, MeetRef, Season, Site};
use census_crawl::net::{FetchOptions, Fetcher};
use census_crawl::recording::RowSink;
use census_crawl::CrawlResult;
use census_domain::model::SourceMeetRef;
use census_domain::UsJurisdiction;
use census_store::Store;
use std::collections::{BTreeMap, HashSet};
use tracing::warn;

use super::{meets_phase, repeats_previous, source_meet_row, MAX_PAGES_PER_SEASON};

/// What one season's pages did, in the shape [`MeetCensus`] reports.
#[derive(Debug, Default)]
pub(super) struct SeasonWalk {
    /// Pages this season's walk read, journaled or not.
    pub(super) pages: usize,
    /// Of those, the ones this run took from the network.
    pub(super) fetched: usize,
    /// Meets the pages published, before de-duplication.
    pub(super) seen: usize,
    /// Pages that repeated their predecessor, ending the season's listing.
    pub(super) repeated: usize,
    /// Set when the per-season page bound cut the walk short.
    pub(super) truncated: usize,
}

/// The fixed inputs of one season's walk: the site, the state and year it is read for, and the
/// observation stamped into every row.
pub(super) struct SeasonReader<'a> {
    pub(super) site: Site,
    pub(super) jurisdiction: UsJurisdiction,
    pub(super) season: Season,
    pub(super) year: u16,
    pub(super) observed_on: &'a str,
    pub(super) refresh: bool,
}

/// Walk one season's index page by page, keeping each page's rows in `rows`.
///
/// `sink` is where the walk's journal markers go: the store it holds on a run that owns its rows, or
/// the caller's recording on a run whose acquisition is posted to an `Ingest` object. The page read
/// and the row building are the same either way; only the marker's destination changes.
pub(super) async fn walk_season(
    fetcher: &Fetcher,
    store: &Store,
    reader: &SeasonReader<'_>,
    rows: &mut BTreeMap<String, SourceMeetRef>,
    sink: &RowSink<'_>,
) -> CrawlResult<SeasonWalk> {
    let phase = meets_phase(reader.jurisdiction, reader.season, reader.year);
    let known = store.journal_keys(&phase)?;
    let mut walk = SeasonWalk::default();
    let mut page = 1_u32;
    let mut previous: Option<Vec<String>> = None;
    loop {
        let read = read_season_page(fetcher, reader, &phase, page, &known, sink).await?;
        walk.pages = walk.pages.saturating_add(1);
        walk.fetched = walk.fetched.saturating_add(usize::from(!read.journaled));
        walk.seen = walk.seen.saturating_add(read.meets.len());
        let ids: Vec<String> = read.meets.iter().map(|meet| meet.meet_id.clone()).collect();
        // The pager's own end signal: a page that repeats its predecessor byte for byte (by meet id)
        // is the site saying "no more", and stopping there is what makes the walk terminate on the
        // site's terms instead of this stage's bound.
        if repeats_previous(previous.as_deref(), &ids) {
            walk.repeated = walk.repeated.saturating_add(1);
            warn_repeated(reader, page);
            break;
        }
        previous = Some(ids);
        keep_rows(rows, &read.meets, reader);
        if !read.has_next {
            break;
        }
        if page >= MAX_PAGES_PER_SEASON {
            // The pager still offered a next page, so this season's census is short of the published
            // set. Counted and logged rather than passed off as a complete read.
            walk.truncated = walk.truncated.saturating_add(1);
            warn_truncated(reader, page);
            break;
        }
        page = page.saturating_add(1);
    }
    Ok(walk)
}

/// One index page: its meets, whether it was already journaled, and whether the pager offered more.
struct SeasonPage {
    journaled: bool,
    meets: Vec<MeetRef>,
    has_next: bool,
}

/// Read one index page, journaling that it was read.
///
/// The journal read that decides whether this page is known stays with the caller: the store is the
/// only surface that holds the phase's keys, and the sink is only where the new marker goes.
async fn read_season_page(
    fetcher: &Fetcher,
    reader: &SeasonReader<'_>,
    phase: &str,
    page: u32,
    known: &HashSet<String>,
    sink: &RowSink<'_>,
) -> CrawlResult<SeasonPage> {
    let key = page.to_string();
    let journaled = !reader.refresh && known.contains(&key);
    let options = FetchOptions {
        refresh: reader.refresh,
        ..Default::default()
    };
    let (meets, has_next) = milesplit::fetch_meet_index(
        fetcher,
        reader.site,
        reader.season,
        reader.year,
        page,
        &options,
    )
    .await?;
    if !journaled {
        // One batch per marker, committed on its own: the marker is the walk's own commit boundary,
        // and a routed run must hold it until the rows it covers are posted.
        let mut batch = sink.write_batch();
        batch.journal_done(phase, &key, &serde_json::json!({ "meets": meets.len() }))?;
        batch.commit()?;
    }
    Ok(SeasonPage {
        journaled,
        meets,
        has_next,
    })
}

/// Keep each page row under its id, first listing wins: a meet published under two seasons is one
/// meet, so a later season cannot relabel a row's identity.
fn keep_rows(
    rows: &mut BTreeMap<String, SourceMeetRef>,
    meets: &[MeetRef],
    reader: &SeasonReader<'_>,
) {
    for meet in meets {
        let row = source_meet_row(
            meet,
            reader.jurisdiction,
            reader.season,
            reader.year,
            reader.observed_on,
        );
        rows.entry(row.id.clone()).or_insert(row);
    }
}

/// The pager served the previous page again: this site's listing ends here.
///
/// Measured on the Ohio walk of 2026-09-22: past its last page the index serves that page again and
/// still advertises a next one, so a walk that trusts `rel="next"` alone never ends.
fn warn_repeated(reader: &SeasonReader<'_>, page: u32) {
    warn!(
        state = reader.jurisdiction.code(),
        season = reader.season.code(),
        year = reader.year,
        page,
        "results index served the previous page again; the season's listing ends here"
    );
}

/// The per-season bound was reached while the pager still offered a next page: the census is short
/// of the published set, which is the one condition this stage must never report silently.
fn warn_truncated(reader: &SeasonReader<'_>, page: u32) {
    warn!(
        state = reader.jurisdiction.code(),
        season = reader.season.code(),
        year = reader.year,
        page,
        "meet index hit the per-season page bound with pages still owed"
    );
}
