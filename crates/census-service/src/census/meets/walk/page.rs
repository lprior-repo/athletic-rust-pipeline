use super::super::{
    progress::{invalid, PageAdmission, PageObservation, SeasonProgress},
    source_meet_row,
};
use super::SeasonReader;
use census_crawl::milesplit::{self, MeetIndexRequest, MeetRef};
use census_crawl::net::Fetcher;
use census_crawl::recording::RowSink;
use census_crawl::CrawlResult;
use census_store::Table;

#[derive(Default)]
pub(super) struct PageMetrics {
    pub(super) pages: usize,
    pub(super) fetched: usize,
    pub(super) seen: usize,
}

pub(super) struct PageContext<'a> {
    pub(super) fetcher: &'a Fetcher,
    pub(super) reader: SeasonReader<'a>,
    pub(super) sink: RowSink<'a>,
    pub(super) phase: &'a str,
}

pub(super) async fn fetch_and_commit(
    input: PageContext<'_>,
    progress: &mut SeasonProgress,
) -> CrawlResult<PageMetrics> {
    let page =
        std::num::NonZeroU32::new(progress.next_page).ok_or_else(|| invalid("zero meet page"))?;
    let request = MeetIndexRequest {
        site: input.reader.site,
        season: input.reader.season,
        year: input.reader.request.year,
        page,
    };
    let fetched =
        milesplit::fetch_meet_index(input.fetcher, &request, input.reader.request.options).await?;
    let observation = progress.observe(&fetched.meets, fetched.continuation)?;
    let seen = fetched.meets.len();
    commit_page(&input, &observation, fetched.meets)?;
    *progress = observation.progress;
    tokio::task::yield_now().await;
    Ok(PageMetrics {
        pages: 1,
        fetched: usize::from(!fetched.from_cache),
        seen,
    })
}

fn commit_page(
    input: &PageContext<'_>,
    observation: &PageObservation,
    meets: Vec<MeetRef>,
) -> CrawlResult<()> {
    let mut batch = input.sink.write_batch();
    if matches!(observation.admission, PageAdmission::Admitted) {
        let rows = project_rows(meets, &input.reader)?;
        batch.append_many(Table::SourceMeets, &rows)?;
    }
    batch.journal_done(input.phase, "cursor", &observation.progress)?;
    batch.commit()
}

fn project_rows(
    meets: Vec<MeetRef>,
    reader: &SeasonReader<'_>,
) -> CrawlResult<Vec<census_domain::model::SourceMeetRef>> {
    let mut rows = Vec::new();
    rows.try_reserve_exact(meets.len())
        .map_err(|_| invalid("cannot reserve bounded meet page"))?;
    rows.extend(meets.into_iter().map(|meet| source_meet_row(meet, reader)));
    Ok(rows)
}
