use std::collections::HashSet;

use census_domain::model::normalize_name;
use census_domain::school_directory::SchoolDirectoryEntry;
use census_domain::UsJurisdiction;
use futures::{stream, StreamExt, TryStreamExt};

use crate::directory::ReadOutcome;
use crate::net::{FetchOptions, FetchOutcome};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};

use super::{map, parse_school_list, DIRECTORY_URL, SOURCE_ID};

mod write;

pub(super) const JOURNAL: &str = "tssaa_schools_v2";

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
    let mut run = Run::new(ctx, options)?.walk().await?;
    let after = ctx.fetcher.stats().await;
    run.report.requests = after.requests.saturating_sub(before.requests);
    run.report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    run.report.note(format!(
        "{} schools committed; {} previously completed schools; {} published appointments",
        run.report.rows, run.skipped, run.coaches
    ));
    Ok(run.report)
}

struct Run<'a> {
    ctx: &'a AdapterContext<'a>,
    options: &'a Options,
    fetch: FetchOptions,
    wanted: HashSet<String>,
    done: HashSet<String>,
    report: AdapterReport,
    skipped: u64,
    coaches: u64,
}

impl<'a> Run<'a> {
    fn new(ctx: &'a AdapterContext<'a>, options: &'a Options) -> CrawlResult<Self> {
        Ok(Self {
            ctx,
            options,
            fetch: FetchOptions {
                refresh: options.refresh || ctx.refresh,
                ..ctx.fetch_options()
            },
            wanted: options
                .school_names
                .iter()
                .map(|name| normalize_name(name))
                .collect(),
            done: ctx.store.journal_keys(JOURNAL)?,
            report: AdapterReport::new(SOURCE_ID, "schools"),
            skipped: 0,
            coaches: 0,
        })
    }

    #[tracing::instrument(skip(self))]
    async fn walk(mut self) -> CrawlResult<Self> {
        if !self.options.states.is_empty()
            && !self.options.states.contains(&UsJurisdiction::Tennessee)
        {
            self.report.note(
                "requested states exclude TN; this association publishes the Tennessee directory",
            );
            return Ok(self);
        }
        if self.options.limit == Some(0) {
            self.report
                .note("limit 0 selects no indexed school records; nothing was fetched");
            return Ok(self);
        }
        let Some(directory) = self.discover().await else {
            return Ok(self);
        };
        let selected = self.select(&directory);
        stream::iter(selected)
            .map(Ok::<_, CrawlError>)
            .try_fold(self, |mut run, row| async move {
                run.process_school(row).await?;
                Ok(run)
            })
            .await
    }

    #[tracing::instrument(skip(self))]
    async fn discover(&mut self) -> Option<ReadOutcome> {
        let capture = self.get(DIRECTORY_URL).await?;
        let text = match std::str::from_utf8(&capture.body) {
            Ok(text) => text,
            Err(error) => {
                self.fail(format!("directory {DIRECTORY_URL}: invalid UTF-8: {error}"));
                return None;
            }
        };
        match parse_school_list(text) {
            Ok(directory) => {
                directory
                    .skipped()
                    .iter()
                    .chain(directory.notes())
                    .for_each(|issue| {
                        self.fail(format!("directory {DIRECTORY_URL}: {}", issue.render()));
                    });
                if directory.entries().is_empty() {
                    self.fail(format!(
                        "directory {DIRECTORY_URL}: no actual indexed schools"
                    ));
                }
                Some(directory)
            }
            Err(error) => {
                self.fail(format!("directory {DIRECTORY_URL}: {error}"));
                None
            }
        }
    }

    fn select<'r>(&mut self, directory: &'r ReadOutcome) -> Vec<&'r SchoolDirectoryEntry> {
        let eligible = |row: &&SchoolDirectoryEntry| {
            !row.address()
                .and_then(|address| address.state())
                .is_some_and(|state| state != UsJurisdiction::Tennessee)
        };
        let names: HashSet<_> = directory
            .entries()
            .iter()
            .filter(eligible)
            .filter_map(|row| row.name())
            .map(|name| normalize_name(name.as_str()))
            .collect();
        self.wanted
            .iter()
            .filter(|name| !names.contains(*name))
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .for_each(|name| self.fail(format!("requested school {name:?} is not indexed in TN")));
        let matching: Vec<_> = directory
            .entries()
            .iter()
            .filter(eligible)
            .filter(|row| {
                self.options.school_names.is_empty()
                    || row
                        .name()
                        .is_some_and(|name| self.wanted.contains(&normalize_name(name.as_str())))
            })
            .collect();
        let limit = self.options.limit.map_or(matching.len(), |limit| limit);
        self.report.note(format!(
            "{} matching indexed TN schools; {} selected by limit",
            matching.len(),
            matching.len().min(limit)
        ));
        matching.into_iter().take(limit).collect()
    }

    #[tracing::instrument(skip(self, row))]
    async fn process_school(&mut self, row: &SchoolDirectoryEntry) -> CrawlResult<()> {
        let id = map::school_id(row)?;
        let key = format!("TN:{}", id.as_str());
        if !self.fetch.refresh && self.done.contains(&key) {
            self.skipped = self.skipped.saturating_add(1);
            return Ok(());
        }
        let url = format!("{DIRECTORY_URL}?id={}", id.as_str());
        let Some(capture) = self.get(&url).await else {
            return Ok(());
        };
        match map::map_capture(row, &capture) {
            Ok(emission) => self.emit(&key, &capture, emission),
            Err(error) => {
                self.fail(format!("school {url}: {error}"));
                Ok(())
            }
        }
    }

    #[tracing::instrument(skip(self))]
    async fn get(&mut self, url: &str) -> Option<FetchOutcome> {
        match self.ctx.fetcher.get(url, &self.fetch).await {
            Ok(capture) if capture.status == 200 => Some(capture),
            Ok(capture) => {
                self.fail(format!("fetch {url}: HTTP {}", capture.status));
                None
            }
            Err(error) => {
                self.fail(format!("fetch {url}: {error}"));
                None
            }
        }
    }

    fn fail(&mut self, message: String) {
        self.report.errors = self.report.errors.saturating_add(1);
        self.report.note(message);
    }
}
