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

#[derive(Debug, Default)]
pub(super) struct SeasonWalk {
    pub(super) pages: usize,
    pub(super) fetched: usize,
    pub(super) seen: usize,
    pub(super) repeated: usize,
    pub(super) truncated: usize,
}

pub(super) struct SeasonReader<'a> {
    pub(super) site: Site,
    pub(super) jurisdiction: UsJurisdiction,
    pub(super) season: Season,
    pub(super) year: u16,
    pub(super) observed_on: &'a str,
    pub(super) refresh: bool,
}

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
            walk.truncated = walk.truncated.saturating_add(1);
            warn_truncated(reader, page);
            break;
        }
        page = page.saturating_add(1);
    }
    Ok(walk)
}

struct SeasonPage {
    journaled: bool,
    meets: Vec<MeetRef>,
    has_next: bool,
}

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

fn warn_repeated(reader: &SeasonReader<'_>, page: u32) {
    warn!(
        state = reader.jurisdiction.code(),
        season = reader.season.code(),
        year = reader.year,
        page,
        "results index served the previous page again; the season's listing ends here"
    );
}

fn warn_truncated(reader: &SeasonReader<'_>, page: u32) {
    warn!(
        state = reader.jurisdiction.code(),
        season = reader.season.code(),
        year = reader.year,
        page,
        "meet index hit the per-season page bound with pages still owed"
    );
}
