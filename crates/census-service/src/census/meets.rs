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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeasonScope {
    Year(u16),
    All,
}

pub const SOURCE: &str = "milesplit";

pub const MAX_PAGES_PER_SEASON: u32 = 400;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeetCensus {
    pub pages: usize,
    pub fetched: usize,
    pub seen: usize,
    pub rows: usize,
    pub seasons: usize,
    pub truncated: usize,
    pub repeated: usize,
    #[serde(default)]
    pub sources: Vec<MeetSourceRows>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeetSourceRows {
    pub slug: String,
    pub rows: usize,
}

impl MeetCensus {
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

pub fn select_meets(
    rows: Vec<SourceMeetRef>,
    states: &[UsJurisdiction],
    scope: SeasonScope,
    limit: Option<usize>,
) -> Vec<SourceMeetRef> {
    let rows = filter_by_season(rows, scope);
    let mut selected: Vec<SourceMeetRef> = rows
        .into_iter()
        .filter(|row| states.is_empty() || states.contains(&row.jurisdiction))
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
fn filter_by_season(rows: Vec<SourceMeetRef>, scope: SeasonScope) -> Vec<SourceMeetRef> {
    match scope {
        SeasonScope::All => rows,
        SeasonScope::Year(year) => rows.into_iter().filter(|row| row.year == year).collect(),
    }
}

fn repeats_previous(previous: Option<&[String]>, current: &[String]) -> bool {
    previous.is_some_and(|previous| previous == current)
}

pub fn meets_phase(jurisdiction: UsJurisdiction, season: Season, year: u16) -> String {
    format!(
        "milesplit_meet_index_{}_{}_{year}_v1",
        jurisdiction.code().to_ascii_lowercase(),
        season.code()
    )
}

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
