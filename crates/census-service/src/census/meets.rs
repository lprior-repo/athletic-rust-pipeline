use census_crawl::milesplit::{MeetRef, Season, Site};
use census_crawl::net::Fetcher;
use census_crawl::recording::RowSink;
use census_crawl::{CrawlResult, Recording};
use census_domain::model::SourceMeetRef;
use census_domain::UsJurisdiction;
use census_store::Store;
use futures::{stream, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tracing::info;

mod progress;
mod request;
mod walk;

pub use progress::DiscoveryDisposition;
pub use request::MeetWalkRequest;

use walk::{walk_season, SeasonReader, SeasonWalk};
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeasonScope {
    Year(u16),
    All,
}

pub const SOURCE: &str = "milesplit";

pub const DEFAULT_PAGES_PER_SEASON: u32 = 400;
pub const MAX_PAGES_PER_INVOCATION: u32 = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeetFrontier {
    pub phase: String,
    pub next_page: u32,
    pub disposition: DiscoveryDisposition,
    #[serde(default)]
    pub failure: Option<MeetFrontierFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeetFrontierFailure {
    pub kind: MeetFrontierFailureKind,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeetFrontierFailureKind {
    Capacity,
    Acquisition,
    Schema,
    Invariant,
}

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
    #[serde(default)]
    pub frontiers: Vec<MeetFrontier>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeetSourceRows {
    pub slug: String,
    pub rows: usize,
    #[serde(default)]
    pub disposition: census_crawl::CollectionDisposition,
    #[serde(default)]
    pub unfinished: Vec<String>,
    #[serde(default)]
    pub errors: Vec<String>,
    #[serde(default)]
    pub notes: Vec<String>,
    #[serde(default)]
    pub withheld: Option<u64>,
    #[serde(default)]
    pub unresolved: Option<census_crawl::UnresolvedCounters>,
}

impl MeetCensus {
    pub fn is_terminal(&self) -> bool {
        !self.sources.is_empty()
            && self.sources.iter().all(|source| {
                source.disposition.is_complete()
                    && source.unfinished.is_empty()
                    && source.errors.is_empty()
                    && source.withheld == Some(0)
                    && source
                        .unresolved
                        .is_some_and(|value| value.rows == 0 && value.labels == 0)
            })
    }

    fn index_terminal(&self) -> bool {
        self.frontiers.len() == Season::ALL.len()
            && self.frontiers.iter().all(|frontier| {
                frontier.disposition == DiscoveryDisposition::Exhausted
                    && frontier.failure.is_none()
            })
    }

    fn completed_index(mut self, site: Site, request: &MeetWalkRequest<'_>) -> Self {
        let disposition = if self.index_terminal() {
            census_crawl::CollectionDisposition::Complete
        } else {
            census_crawl::CollectionDisposition::Partial
        };
        let unfinished = self
            .frontiers
            .iter()
            .zip(Season::ALL)
            .filter(|(frontier, _)| frontier.disposition != DiscoveryDisposition::Exhausted)
            .map(|(frontier, season)| site.results_url(season, request.year, frontier.next_page))
            .collect();
        let errors = self
            .frontiers
            .iter()
            .filter_map(|frontier| {
                frontier
                    .failure
                    .as_ref()
                    .map(|failure| failure.detail.clone())
            })
            .collect();
        self.sources.push(MeetSourceRows {
            slug: SOURCE.to_owned(),
            rows: self.rows,
            disposition,
            unfinished,
            errors,
            notes: Vec::new(),
            withheld: Some(0),
            unresolved: self
                .index_terminal()
                .then_some(census_crawl::UnresolvedCounters { rows: 0, labels: 0 }),
        });
        self
    }

    fn absorb(mut self, walk: SeasonWalk) -> CrawlResult<Self> {
        self.pages = walk::add(self.pages, walk.pages)?;
        self.fetched = walk::add(self.fetched, walk.fetched)?;
        self.seen = walk::add(self.seen, walk.seen)?;
        self.rows = walk::add(self.rows, walk.rows)?;
        self.repeated = walk::add(self.repeated, walk.repeated)?;
        self.truncated = walk::add(self.truncated, walk.truncated)?;
        self.seasons = walk::add(self.seasons, 1)?;
        if let Some(frontier) = walk.frontier {
            self.frontiers.push(frontier);
        }
        Ok(self)
    }
}

pub async fn collect_state_meets(
    fetcher: &Fetcher,
    store: &Store,
    request: &MeetWalkRequest<'_>,
    recording: Option<&Recording>,
) -> CrawlResult<MeetCensus> {
    let site = Site::for_jurisdiction(request.jurisdiction);
    let sink = recording.map_or(RowSink::Store(store), |recording| RowSink::Record {
        store,
        recording,
    });
    let mut initial = MeetCensus::default();
    initial
        .frontiers
        .try_reserve_exact(Season::ALL.len())
        .map_err(|_| progress::invalid("cannot reserve meet frontiers"))?;
    let census = stream::iter(Season::ALL.into_iter().map(Ok))
        .try_fold(initial, |census, season| async move {
            let reader = SeasonReader {
                site,
                season,
                request,
            };
            census.absorb(walk_season(fetcher, store, reader, sink).await?)
        })
        .await?;
    info!(
        state = request.jurisdiction.code(),
        pages = census.pages,
        seen = census.seen,
        rows = census.rows,
        "meet census collected"
    );
    Ok(census.completed_index(site, request))
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

pub fn meets_phase(jurisdiction: UsJurisdiction, season: Season, year: u16) -> String {
    format!(
        "milesplit_meet_index_{}_{}_{year}_v2",
        jurisdiction.code().to_ascii_lowercase(),
        season.code()
    )
}

fn source_meet_row(meet: MeetRef, reader: &SeasonReader<'_>) -> SourceMeetRef {
    SourceMeetRef {
        id: SourceMeetRef::row_id(SOURCE, &meet.meet_id),
        source: SOURCE.to_string(),
        source_meet_id: meet.meet_id,
        jurisdiction: reader.request.jurisdiction,
        season: reader.season.code().to_string(),
        year: reader.request.year,
        name: meet.name,
        date: meet.date,
        venue: meet.venue,
        results_url: meet.results_url,
        observed_on: reader.request.observed_on.to_string(),
    }
}
#[cfg(test)]
#[path = "meets/tests.rs"]
mod tests;
