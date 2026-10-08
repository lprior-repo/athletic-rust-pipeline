mod write;

use super::map::{coach_from_row, school_entities, CoachRow, SchoolExtract, SchoolTable};
use super::pages::checked::{visit_capture, DirectoryRecord};
use super::Options;
use crate::directory::acquisition::{fail, owe, publish};
use crate::net::{FetchOptions, FetchOutcome};
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_store::Table;

struct DirectoryRun<'a> {
    ctx: &'a AdapterContext<'a>,
    options: &'a Options,
    capture: &'a FetchOutcome,
    report: &'a mut AdapterReport,
    processed: usize,
    owner: Option<SchoolExtract>,
    coaches: usize,
    complete: bool,
}

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    collect_directory(ctx, options, &format!("{}/Directory.aspx", super::HOST)).await
}

pub(super) async fn collect_directory(
    ctx: &AdapterContext<'_>,
    options: &Options,
    url: &str,
) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("riil", "schools");
    let before = ctx.fetcher.stats().await;
    let fetch = FetchOptions {
        refresh: options.refresh || ctx.refresh,
        ..ctx.fetch_options()
    };
    match ctx.fetcher.get(url, &fetch).await {
        Ok(capture) => apply_directory(ctx, options, &capture, &mut report)?,
        Err(error) => fail(&mut report, url, error)?,
    }
    if report.unfinished.is_empty() {
        report.finish_frontier();
    }
    let after = ctx.fetcher.stats().await;
    report.requests = after
        .physical_requests()
        .saturating_sub(before.physical_requests());
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    Ok(report)
}

fn apply_directory(
    ctx: &AdapterContext<'_>,
    options: &Options,
    capture: &FetchOutcome,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    let mut run = DirectoryRun {
        ctx,
        options,
        capture,
        report,
        processed: 0,
        owner: None,
        coaches: 0,
        complete: true,
    };
    visit_capture(capture, |locator, parsed| run.apply(locator, parsed))
}

impl DirectoryRun<'_> {
    fn apply(&mut self, locator: &str, parsed: CrawlResult<DirectoryRecord>) -> CrawlResult<()> {
        match parsed {
            Ok(DirectoryRecord::School(table)) => self.school(locator, &table),
            Ok(DirectoryRecord::Coach(row)) => self.coach(locator, &row),
            Ok(DirectoryRecord::Complete) => {
                if self.complete {
                    if let Some(owner) = &self.owner {
                        write::complete(self.ctx, &owner.school, self.capture, self.coaches)?;
                    }
                }
                Ok(())
            }
            Err(error) => {
                self.complete = false;
                fail(self.report, locator, error)
            }
        }
    }

    fn school(&mut self, locator: &str, table: &SchoolTable) -> CrawlResult<()> {
        self.owner = None;
        self.coaches = 0;
        if self
            .options
            .limit
            .is_some_and(|limit| self.processed >= limit)
        {
            return owe(self.report, locator);
        }
        self.complete = true;
        self.processed = self.processed.saturating_add(1);
        let extract = school_entities(table, self.capture);
        let before = self.report.errors;
        if write::owner(self.ctx, &extract, self.capture, locator, self.report)? {
            self.report.rows = self.report.rows.saturating_add(1);
        }
        self.owner = Some(extract);
        self.complete = before == self.report.errors;
        Ok(())
    }

    fn coach(&mut self, locator: &str, row: &CoachRow) -> CrawlResult<()> {
        let Some(owner) = &self.owner else {
            return Ok(());
        };
        let coach = coach_from_row(&owner.school.id, row, self.capture);
        let before = self.report.errors;
        publish(
            self.ctx,
            ("riil", locator),
            Table::Coaches,
            std::slice::from_ref(&coach),
            self.report,
        )?;
        self.coaches = self.coaches.saturating_add(1);
        self.complete &= before == self.report.errors;
        Ok(())
    }
}
