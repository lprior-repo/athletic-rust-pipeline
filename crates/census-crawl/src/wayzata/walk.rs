use super::budget;
use super::map;
use super::parse::{schedule_url, MeetRow, ScheduleSport};
use super::projection::{self, Page};
use super::receipt;
use super::results::walk::ResultsWalk;
use futures::{stream, TryStreamExt};
use super::{ADAPTER_ID, Options, stats_of};
use crate::{AdapterContext, AdapterReport, CollectionDisposition, CrawlError, CrawlResult};
pub(super) struct Walk {
    report: AdapterReport,
    completed: usize,
    pages: u64,
}

struct Located {
    ordinal: usize,
    row: MeetRow,
}

impl Walk {
    pub(super) fn new() -> Self {
        Self {
            report: AdapterReport::new(ADAPTER_ID, "meet-schedule rows"),
            completed: 0,
            pages: 0,
        }
    }

    #[tracing::instrument(skip(self, ctx, options, years))]
    pub(super) async fn run(
        self,
        ctx: &AdapterContext<'_>,
        options: &Options,
        years: &[i16],
    ) -> CrawlResult<Self> {
        let pages = [ScheduleSport::Track, ScheduleSport::CrossCountry]
            .into_iter()
            .flat_map(|sport| years.iter().copied().map(move |year| (sport, year)));
        stream::iter(pages.map(Ok))
            .try_fold(self, |mut walk, (sport, year)| async move {
                let url = schedule_url(sport, year);
                if walk.limited(options) {
                    walk.owed(url)?;
                    return Ok(walk);
                }
                if let Err(error) = walk.read_page(ctx, options, sport, year).await {
                    walk.failed(url, error)?;
                }
                Ok(walk)
            })
            .await
    }

    fn limited(&self, options: &Options) -> bool {
        options.limit.is_some_and(|limit| self.completed >= limit)
    }

    #[tracing::instrument(skip(self, ctx, options))]
    async fn read_page(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &Options,
        sport: ScheduleSport,
        year: i16,
    ) -> CrawlResult<()> {
        let url = schedule_url(sport, year);
        let mut fetch_options = ctx.fetch_options();
        fetch_options.refresh = ctx.refresh || options.refresh;
        let fetched = ctx.fetcher.get(&url, &fetch_options).await?;
        let body = validate_page(&fetched, &url)?;
        let rows = super::parse::rows(body, year)?;
        self.pages = budget::add(self.pages, 1)?;
        let page = Page {
            fetched: &fetched,
            sport,
            year,
        };
        self.apply_rows(ctx, options, &page, rows).await
    }

    async fn apply_rows(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &Options,
        page: &Page<'_>,
        rows: Vec<MeetRow>,
    ) -> CrawlResult<()> {
        for (index, row) in rows.into_iter().enumerate() {
            let ordinal = index
                .checked_add(1)
                .ok_or_else(|| budget::arithmetic("Wayzata row ordinal"))?;
            let located = Located { ordinal, row };
            if let Err(error) = self.apply_row(ctx, options, page, &located) {
                self.failed(locator(page, ordinal), error)?;
            }
            self.collect_results(ctx, &located).await?;
        }
        Ok(())
    }

    fn apply_row(
        &mut self,
        ctx: &AdapterContext<'_>,
        options: &Options,
        page: &Page<'_>,
        located: &Located,
    ) -> CrawlResult<()> {
        receipt::retain_raw(ctx, page, located.ordinal, &located.row)?;
        let effect = receipt::projection_effect(ctx, options, page, located.ordinal)?;
        if ctx.effect_is_committed(&effect.operation, &effect.digest)? {
            return Ok(());
        }
        let meet = projection::project(ctx, options, page, &located.row)?;
        if meet.is_some() && self.limited(options) {
            return self.owed(locator(page, located.ordinal));
        }
        if receipt::commit_projection(ctx, &effect, meet.as_ref())? && meet.is_some() {
            self.completed = self
                .completed
                .checked_add(1)
                .ok_or_else(|| budget::arithmetic("Wayzata completed rows"))?;
            self.report.rows = budget::add(self.report.rows, 1)?;
        }
        Ok(())
    }

    fn owed(&mut self, locator: String) -> CrawlResult<()> {
        budget::reserve(&mut self.report.unfinished, 1, budget::MAX_UNFINISHED)?;
        self.report.unfinished.push(locator);
        self.report.disposition = CollectionDisposition::Partial;
        Ok(())
    }

    fn failed(&mut self, locator: String, error: CrawlError) -> CrawlResult<()> {
        self.owed(locator)?;
        self.report.errors = budget::add(self.report.errors, 1)?;
        if self.report.notes.len() < 5 {
            budget::reserve(&mut self.report.notes, 1, 6)?;
            self.report.note(budget::detail(&error)?);
        }
        Ok(())
    }

    async fn collect_results(&mut self, ctx: &AdapterContext<'_>, located: &Located) -> CrawlResult<()> {
        if located.row.slug.is_none() {
            return Ok(());
        }
        let slug = located.row.slug.as_ref().unwrap();
        let mut results = ResultsWalk::new();
        results.collect_for_slug(ctx, slug, &located.row.name, &located.row.date).await?;
        self.report.rows = budget::add(self.report.rows, results.collected as u64)?;
        Ok(())
    }

    #[tracing::instrument(skip(self, ctx))]
    pub(super) async fn finish(
        mut self,
        ctx: &AdapterContext<'_>,
        before: (u64, u64),
    ) -> CrawlResult<AdapterReport> {
        let after = stats_of(ctx).await;
        self.report.requests = budget::delta(after.0, before.0)?;
        self.report.from_cache = budget::delta(after.1, before.1)?;
        budget::reserve(&mut self.report.notes, 1, 6)?;
        self.report.note(format!(
            "schedules: {} pages read; schedule metadata only, no performance/result projection",
            self.pages
        ));
        if self.report.unfinished.is_empty() {
            self.report.finish_frontier();
        }
        Ok(self.report)
    }
}

fn locator(page: &Page<'_>, ordinal: usize) -> String {
    format!("{}#row={ordinal}", page.fetched.url)
}

fn validate_page<'a>(fetched: &'a crate::net::FetchOutcome, url: &str) -> CrawlResult<&'a str> {
    budget::check(
        "Wayzata schedule bytes",
        fetched.body.len(),
        budget::MAX_PAGE_BYTES,
    )?;
    validate_capture(fetched, url)?;
    let body = std::str::from_utf8(&fetched.body).map_err(|error| CrawlError::Schema {
        url: fetched.url.clone(),
        detail: format!("schedule is not UTF-8: {error}"),
    })?;
    super::parse::schedule_body(body)?.ok_or_else(|| CrawlError::Schema {
        url: fetched.url.clone(),
        detail: "published schedule table is missing".to_string(),
    })
}

fn validate_capture(fetched: &crate::net::FetchOutcome, url: &str) -> CrawlResult<()> {
    let physical_date = chrono::DateTime::parse_from_rfc3339(&fetched.fetched_at).is_ok()
        || (fetched.fetched_at.len() == 10
            && chrono::NaiveDate::parse_from_str(&fetched.fetched_at, "%Y-%m-%d").is_ok());
    let digest = fetched.content_digest.len() == 64
        && fetched
            .content_digest
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit());
    if !physical_date
        || !digest
        || fetched.url != url
        || fetched.method != "GET"
        || !(200..300).contains(&fetched.status)
        || fetched.status == 206
        || fetched.bytes != fetched.body.len()
    {
        return Err(CrawlError::Schema {
            url: url.to_string(),
            detail: "physical capture metadata is missing or inconsistent".to_string(),
        });
    }
    Ok(())
}
