use super::progress::{invalid, DiscoveryDisposition, SeasonProgress};
use super::{
    meets_phase, MeetFrontier, MeetFrontierFailure, MeetFrontierFailureKind, MeetWalkRequest,
};
use census_crawl::milesplit::{Season, Site};
use census_crawl::net::Fetcher;
use census_crawl::recording::RowSink;
use census_crawl::CrawlResult;
use census_store::Store;
use futures::{stream, TryStreamExt};
use page::{PageContext, PageMetrics};

mod page;

#[derive(Default)]
pub(super) struct SeasonWalk {
    pub(super) pages: usize,
    pub(super) fetched: usize,
    pub(super) seen: usize,
    pub(super) rows: usize,
    pub(super) repeated: usize,
    pub(super) truncated: usize,
    pub(super) frontier: Option<MeetFrontier>,
}

#[derive(Clone, Copy)]
pub(super) struct SeasonReader<'a> {
    pub(super) site: Site,
    pub(super) season: Season,
    pub(super) request: &'a MeetWalkRequest<'a>,
}

struct PageState<'a> {
    fetcher: &'a Fetcher,
    reader: SeasonReader<'a>,
    sink: RowSink<'a>,
    phase: String,
    progress: SeasonProgress,
    remaining: u32,
}

enum WalkStep<'a> {
    Read(PageState<'a>),
    Done,
}

struct PageStep {
    metrics: PageMetrics,
    completed: Option<SeasonWalk>,
}

pub(super) async fn walk_season<'a>(
    fetcher: &'a Fetcher,
    store: &'a Store,
    reader: SeasonReader<'a>,
    sink: RowSink<'a>,
) -> CrawlResult<SeasonWalk> {
    let phase = meets_phase(
        reader.request.jurisdiction,
        reader.season,
        reader.request.year,
    );
    let progress = SeasonProgress::load(store, &phase)?;
    let remaining = reader.request.page_budget;
    let initial = WalkStep::Read(PageState {
        fetcher,
        reader,
        sink,
        phase,
        progress,
        remaining,
    });
    stream::try_unfold(initial, next_step)
        .try_fold(SeasonWalk::default(), |walk, step| {
            futures::future::ready(walk.absorb(step))
        })
        .await
}

async fn next_step<'a>(state: WalkStep<'a>) -> CrawlResult<Option<(PageStep, WalkStep<'a>)>> {
    match state {
        WalkStep::Done => Ok(None),
        WalkStep::Read(state) => state.advance().await.map(Some),
    }
}

impl<'a> PageState<'a> {
    async fn advance(mut self) -> CrawlResult<(PageStep, WalkStep<'a>)> {
        if self.progress.disposition == DiscoveryDisposition::Exhausted || self.remaining == 0 {
            return Ok(self.finish(PageMetrics::default(), None));
        }
        let input = PageContext {
            fetcher: self.fetcher,
            reader: self.reader,
            sink: self.sink,
            phase: &self.phase,
        };
        let metrics = match page::fetch_and_commit(input, &mut self.progress).await {
            Ok(metrics) => metrics,
            Err(error) => return Ok(self.failed(error)),
        };
        self.remaining = self
            .remaining
            .checked_sub(1)
            .ok_or_else(|| invalid("meet page budget underflow"))?;
        if self.progress.disposition != DiscoveryDisposition::Partial || self.remaining == 0 {
            return Ok(self.finish(metrics, None));
        }
        Ok((
            PageStep {
                metrics,
                completed: None,
            },
            WalkStep::Read(self),
        ))
    }

    fn finish(
        self,
        metrics: PageMetrics,
        failure: Option<MeetFrontierFailure>,
    ) -> (PageStep, WalkStep<'a>) {
        let repeated = usize::from(self.progress.disposition == DiscoveryDisposition::Quarantined);
        let truncated = usize::from(self.progress.disposition == DiscoveryDisposition::Partial);
        let frontier = MeetFrontier {
            phase: self.phase,
            next_page: self.progress.next_page,
            disposition: self.progress.disposition,
            failure,
        };
        let completed = SeasonWalk {
            rows: self.progress.rows_seen,
            repeated,
            truncated,
            frontier: Some(frontier),
            ..Default::default()
        };
        (
            PageStep {
                metrics,
                completed: Some(completed),
            },
            WalkStep::Done,
        )
    }

    fn failed(mut self, error: census_crawl::CrawlError) -> (PageStep, WalkStep<'a>) {
        use census_crawl::CrawlError;
        let kind = match &error {
            CrawlError::Resource { .. } => MeetFrontierFailureKind::Capacity,
            CrawlError::Fetch(_) | CrawlError::Io { .. } => MeetFrontierFailureKind::Acquisition,
            CrawlError::Schema { .. } | CrawlError::Decode { .. } => {
                MeetFrontierFailureKind::Schema
            }
            _ => MeetFrontierFailureKind::Invariant,
        };
        if kind == MeetFrontierFailureKind::Schema {
            self.progress.disposition = DiscoveryDisposition::Quarantined;
        }
        let failure = MeetFrontierFailure {
            kind,
            detail: bounded_failure(error.to_string()),
        };
        self.finish(PageMetrics::default(), Some(failure))
    }
}

impl SeasonWalk {
    fn absorb(mut self, step: PageStep) -> CrawlResult<Self> {
        self.pages = add(self.pages, step.metrics.pages)?;
        self.fetched = add(self.fetched, step.metrics.fetched)?;
        self.seen = add(self.seen, step.metrics.seen)?;
        if let Some(completed) = step.completed {
            self.rows = completed.rows;
            self.repeated = completed.repeated;
            self.truncated = completed.truncated;
            self.frontier = completed.frontier;
        }
        Ok(self)
    }
}

pub(super) fn add(left: usize, right: usize) -> CrawlResult<usize> {
    left.checked_add(right)
        .ok_or_else(|| invalid("meet census counter overflow"))
}

fn bounded_failure(mut detail: String) -> String {
    if detail.len() > 4096 {
        let boundary = detail
            .char_indices()
            .map(|(index, _)| index)
            .take_while(|index| *index <= 4096)
            .last()
            .map_or(0, core::convert::identity);
        detail.truncate(boundary);
    }
    detail
}
