use super::map::{map_coach_row, map_directory_row};
use super::parse::{parse_directory, parse_school_page, MemberSchool};
use super::{nonempty, school_page_url, DIRECTORY_URL, SOURCE_ID};
use crate::directory::acquisition::{
    fail, owe, publish as persist, publish_school as persist_school, text,
};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{normalize_name, SourceNamespace};
use census_domain::UsJurisdiction;
use census_store::Table;
use futures::{stream, StreamExt, TryStreamExt};

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

struct DirectoryCapture {
    rows: Vec<MemberSchool>,
    fetched_at: String,
}

struct Run<'a> {
    ctx: &'a AdapterContext<'a>,
    options: &'a Options,
    report: AdapterReport,
    processed: usize,
}

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let before = ctx.fetcher.stats().await;
    let run = Run {
        ctx,
        options,
        report: AdapterReport::new(SOURCE_ID, "schools"),
        processed: 0,
    };
    if !options.states.is_empty() && !options.states.contains(&UsJurisdiction::Colorado) {
        return Ok(run.report);
    }
    let mut run = run.schools().await?;
    if run.report.unfinished.is_empty() {
        run.report.finish_frontier();
    }
    let after = ctx.fetcher.stats().await;
    run.report.requests = after
        .physical_requests()
        .saturating_sub(before.physical_requests());
    run.report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    Ok(run.report)
}

impl Run<'_> {
    async fn schools(mut self) -> CrawlResult<Self> {
        let Some(capture) = self.directory().await? else {
            return Ok(self);
        };
        let (run, _) = stream::iter(capture.rows.into_iter().enumerate())
            .map(Ok::<_, CrawlError>)
            .try_fold(
                (self, capture.fetched_at),
                |(mut run, fetched_at), (index, row)| async move {
                    run.school(index, &row, &fetched_at).await?;
                    Ok((run, fetched_at))
                },
            )
            .await?;
        Ok(run)
    }

    async fn directory(&mut self) -> CrawlResult<Option<DirectoryCapture>> {
        let fetch = crate::net::FetchOptions {
            refresh: self.ctx.refresh || self.options.refresh,
            ..self.ctx.fetch_options()
        };
        let result = match self.ctx.fetcher.get(DIRECTORY_URL, &fetch).await {
            Ok(capture) => text(&capture)
                .and_then(parse_directory)
                .map(|rows| DirectoryCapture {
                    rows,
                    fetched_at: capture.fetched_at,
                }),
            Err(error) => Err(error.into()),
        };
        match result {
            Ok(capture) if !capture.rows.is_empty() => Ok(Some(capture)),
            Ok(_) => {
                owe(&mut self.report, DIRECTORY_URL)?;
                Ok(None)
            }
            Err(error) => {
                fail(&mut self.report, DIRECTORY_URL, error)?;
                Ok(None)
            }
        }
    }

    async fn school(
        &mut self,
        index: usize,
        row: &MemberSchool,
        directory_stamp: &str,
    ) -> CrawlResult<()> {
        if !self.options.school_names.is_empty()
            && !row.name.as_deref().is_some_and(|name| {
                self.options
                    .school_names
                    .iter()
                    .any(|wanted| normalize_name(wanted) == normalize_name(name))
            })
        {
            return Ok(());
        }
        let Some(slug) = row.slug.as_deref().and_then(nonempty) else {
            self.report.rejections = self.report.rejections.saturating_add(1);
            return owe(&mut self.report, format!("{DIRECTORY_URL}#row={index}"));
        };
        let url = school_page_url(&slug);
        if self
            .options
            .limit
            .is_some_and(|limit| self.processed >= limit)
        {
            return owe(&mut self.report, url);
        }
        let capture = match self.ctx.fetcher.get(&url, &self.ctx.fetch_options()).await {
            Ok(capture) => capture,
            Err(error) => return fail(&mut self.report, &url, error),
        };
        self.emit_school(index, row, &url, &capture, directory_stamp)
    }

    fn emit_school(
        &mut self,
        index: usize,
        row: &MemberSchool,
        url: &str,
        capture: &crate::net::FetchOutcome,
        directory_stamp: &str,
    ) -> CrawlResult<()> {
        let Some((school, id)) = map_directory_row(row, DIRECTORY_URL, directory_stamp) else {
            self.report.rejections = self.report.rejections.saturating_add(1);
            return owe(&mut self.report, format!("{DIRECTORY_URL}#row={index}"));
        };
        let written = persist_school(
            self.ctx,
            (SOURCE_ID, url),
            (
                &SourceNamespace::association_school(SOURCE_ID),
                &school,
                directory_stamp,
            ),
            &mut self.report,
        )?;
        let rows = match text(capture).and_then(parse_school_page) {
            Ok(rows) => rows,
            Err(error) => return fail(&mut self.report, url, error),
        };
        rows.iter()
            .try_for_each(|row| self.emit_coach(&id, url, capture, row))?;
        self.processed = self.processed.saturating_add(1);
        self.report.rows = self
            .report
            .rows
            .saturating_add(u64::try_from(written).map_err(|_| CrawlError::Arithmetic {
                detail: "school count".into(),
            })?);
        Ok(())
    }

    fn emit_coach(
        &mut self,
        id: &census_domain::model::SchoolId,
        url: &str,
        capture: &crate::net::FetchOutcome,
        row: &super::parse::SchoolCoachRow,
    ) -> CrawlResult<()> {
        let Some(mut coach) = map_coach_row(row, id, url, &capture.fetched_at) else {
            return Ok(());
        };
        crate::coach_directories::record_appointment(
            &mut coach,
            Some(crate::coach_directories::PublishedListing {
                role: &row.title,
                program: &row.activity_name,
                source: SOURCE_ID,
                school_year: self.ctx.school_year,
            }),
            crate::coach_directories::StaffCapture {
                url: &capture.url,
                observed_on: &capture.fetched_at,
                sha256: &capture.content_digest,
            },
        )?;
        persist(
            self.ctx,
            (SOURCE_ID, url),
            Table::Coaches,
            std::slice::from_ref(&coach),
            &mut self.report,
        )
        .map(|_| ())
    }
}
