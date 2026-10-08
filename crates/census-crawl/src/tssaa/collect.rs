use census_domain::model::{normalize_name, CoachTenure};
use census_domain::school_directory::SchoolDirectoryEntry;
use census_domain::UsJurisdiction;
use futures::{stream, StreamExt, TryStreamExt};

use super::{map, parse_school_list, DIRECTORY_URL, SOURCE_ID};
use crate::directory::ReadOutcome;
use crate::net::{FetchOptions, FetchOutcome};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};

mod write;
pub(super) const JOURNAL: &str = "tssaa_school_projection_v3";

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

#[tracing::instrument(skip(ctx, options))]
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let before = ctx.fetcher.stats().await;
    let mut run = Run::new(ctx, options).walk().await?;
    let after = ctx.fetcher.stats().await;
    run.report.requests = after
        .physical_requests()
        .checked_sub(before.physical_requests())
        .ok_or_else(counter_error)?;
    run.report.from_cache = after
        .cache_hits
        .checked_sub(before.cache_hits)
        .ok_or_else(counter_error)?;
    run.report.note(format!(
        "{} schools committed; {} qualified replays; {} published appointments",
        run.report.rows, run.skipped, run.coaches
    ));
    Ok(run.report)
}

fn counter_error() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "TSSAA counter overflow or reversal".to_string(),
    }
}

struct Run<'a> {
    ctx: &'a AdapterContext<'a>,
    options: &'a Options,
    fetch: FetchOptions,
    report: AdapterReport,
    skipped: u64,
    coaches: u64,
}

impl<'a> Run<'a> {
    fn new(ctx: &'a AdapterContext<'a>, options: &'a Options) -> Self {
        Self {
            ctx,
            options,
            fetch: FetchOptions {
                refresh: options.refresh || ctx.refresh,
                ..ctx.fetch_options()
            },
            report: AdapterReport::new(SOURCE_ID, "schools"),
            skipped: 0,
            coaches: 0,
        }
    }

    #[tracing::instrument(skip(self))]
    async fn walk(mut self) -> CrawlResult<Self> {
        if !self.options.states.is_empty()
            && !self.options.states.contains(&UsJurisdiction::Tennessee)
        {
            self.report.note("requested states exclude TN");
            return Ok(self);
        }
        let Some(directory) = self.discover().await? else {
            return Ok(self);
        };
        self.validate_requested(&directory)?;
        let options = self.options;
        let mut run = stream::iter(
            directory
                .entries()
                .iter()
                .filter(|row| matches(options, row))
                .enumerate(),
        )
        .map(Ok::<_, CrawlError>)
        .try_fold(self, |mut run, (index, row)| async move {
            if run.options.limit.is_some_and(|limit| index >= limit) {
                run.owe(&school_url(row)?)?;
            } else {
                run.process_school(row).await?;
            }
            Ok(run)
        })
        .await?;
        run.report.finish_frontier();
        Ok(run)
    }

    fn validate_requested(&mut self, directory: &ReadOutcome) -> CrawlResult<()> {
        let options = self.options;
        options.school_names.iter().try_for_each(|wanted| {
            if directory.entries().iter().any(|row| {
                row.name()
                    .is_some_and(|name| normalize_name(wanted) == normalize_name(name.as_str()))
            }) {
                return Ok(());
            }
            self.fail(
                DIRECTORY_URL,
                format!("requested school {wanted:?} is not indexed"),
            )
        })
    }

    #[tracing::instrument(skip(self))]
    async fn discover(&mut self) -> CrawlResult<Option<ReadOutcome>> {
        let Some(capture) = self.get(DIRECTORY_URL).await? else {
            return Ok(None);
        };
        let parsed = std::str::from_utf8(&capture.body)
            .map_err(|error| error.to_string())
            .and_then(|text| parse_school_list(text).map_err(|error| error.to_string()));
        let directory = match parsed {
            Ok(directory) => directory,
            Err(error) => {
                self.fail(DIRECTORY_URL, error)?;
                return Ok(None);
            }
        };
        directory
            .skipped()
            .iter()
            .try_for_each(|issue| self.fail(DIRECTORY_URL, issue.render()))?;
        directory
            .notes()
            .iter()
            .take(5)
            .for_each(|issue| self.report.note(issue.render()));
        Ok(Some(directory))
    }

    #[tracing::instrument(skip(self, row))]
    async fn process_school(&mut self, row: &SchoolDirectoryEntry) -> CrawlResult<()> {
        let url = school_url(row)?;
        let Some(capture) = self.get(&url).await? else {
            return Ok(());
        };
        let mut emission = match map::map_capture(row, &capture) {
            Ok(emission) => emission,
            Err(error) => {
                self.fail(&url, error.to_string())?;
                return Ok(());
            }
        };
        if emission.coaches.iter().any(|coach| !coach.tenure_evidence.iter().any(|tenure| {
            matches!(tenure.tenure, CoachTenure::Current { school_year } if school_year == self.ctx.school_year)
        })) {
            emission.issues.try_reserve(1).map_err(|_| CrawlError::Resource { resource: "TSSAA season issue", requested: 1, limit: 8192 })?;
            emission.issues.push("appointment season is missing or differs from requested school year".to_string());
        }
        let outcome = if emission.issues.is_empty() {
            census_domain::model::ContactResearchOutcome::CompletedEmpty
        } else {
            census_domain::model::ContactResearchOutcome::Partial
        };
        crate::coach_directories::persist_staff_capture(
            self.ctx,
            &emission.school,
            &capture,
            &emission.coaches,
            outcome,
        )?;
        let key = Self::projection_key(&capture);
        self.emit(&key, &capture, emission)
    }

    fn projection_key(capture: &FetchOutcome) -> String {
        census_domain::model::Id::<()>::mint(
            JOURNAL,
            &[&capture.url, &capture.content_digest, &capture.fetched_at],
        )
        .to_string()
    }

    #[tracing::instrument(skip(self))]
    async fn get(&mut self, url: &str) -> CrawlResult<Option<FetchOutcome>> {
        match self.ctx.fetcher.get(url, &self.fetch).await {
            Ok(capture) if capture.status == 200 => {
                if capture.body.len() > 1024 * 1024
                    || capture
                        .body
                        .windows(3)
                        .filter(|part| *part == b"<tr")
                        .count()
                        > 8192
                {
                    self.fail(
                        url,
                        "published capture exceeds bounded directory capacity".to_string(),
                    )?;
                    return Ok(None);
                }
                Ok(Some(capture))
            }
            Ok(capture) => {
                self.fail(url, format!("HTTP {}", capture.status))?;
                Ok(None)
            }
            Err(error) => {
                self.fail(url, error.to_string())?;
                Ok(None)
            }
        }
    }

    fn owe(&mut self, url: &str) -> CrawlResult<()> {
        if self.report.unfinished.iter().any(|value| value == url) {
            return Ok(());
        }
        self.report
            .unfinished
            .try_reserve(1)
            .map_err(|_| CrawlError::Resource {
                resource: "TSSAA unfinished locators",
                requested: 1,
                limit: crate::net::MAX_BODY_BYTES,
            })?;
        self.report.unfinished.push(url.to_string());
        Ok(())
    }

    fn fail(&mut self, url: &str, message: String) -> CrawlResult<()> {
        self.report.errors = self
            .report
            .errors
            .checked_add(1)
            .ok_or_else(counter_error)?;
        self.owe(url)?;
        if self.report.errors <= 5 {
            self.report.note(
                format!("{url}: {message}")
                    .chars()
                    .take(4096)
                    .collect::<String>(),
            );
        }
        Ok(())
    }
}

fn school_url(row: &SchoolDirectoryEntry) -> CrawlResult<String> {
    Ok(format!(
        "{DIRECTORY_URL}?id={}",
        map::school_id(row)?.as_str()
    ))
}

fn matches(options: &Options, row: &SchoolDirectoryEntry) -> bool {
    !row.address()
        .and_then(|address| address.state())
        .is_some_and(|state| state != UsJurisdiction::Tennessee)
        && (options.school_names.is_empty()
            || row.name().is_some_and(|name| {
                options
                    .school_names
                    .iter()
                    .any(|wanted| normalize_name(wanted) == normalize_name(name.as_str()))
            }))
}
