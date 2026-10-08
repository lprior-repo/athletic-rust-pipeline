use super::journal::Journal;
use super::map::Mapper;
use super::report::{narrate, Walked};
use super::requests::{self, events_url};
use super::wire::{EventRow, MeetRow};
use super::{parse, xc};
use crate::ihsa::Options;
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::UsJurisdiction;

mod events;

const SOURCE: &str = "ihsa";

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let before = ctx.fetcher.stats().await;
    let mut run = Run::new(ctx, options)?;
    run.requests_before = before.physical_requests();
    run.cache_before = before.cache_hits;
    if !run.in_scope() {
        return Ok(run.report);
    }
    run.walk().await?;
    run.finish().await
}

struct Run<'a> {
    ctx: &'a AdapterContext<'a>,
    options: &'a Options,
    mapper: Mapper<'a>,
    report: AdapterReport,
    journal: Journal,
    requests_before: u64,
    cache_before: u64,
    walked: usize,
    skipped_meets: usize,
    resumed_lists: usize,
    summaries: usize,
}

impl<'a> Run<'a> {
    fn new(ctx: &'a AdapterContext<'a>, options: &'a Options) -> CrawlResult<Self> {
        Ok(Self {
            ctx,
            options,
            mapper: Mapper::load(ctx, SOURCE)?,
            report: AdapterReport::new(SOURCE, "performances"),
            journal: Journal::load(ctx)?,
            requests_before: 0,
            cache_before: 0,
            walked: 0,
            skipped_meets: 0,
            resumed_lists: 0,
            summaries: 0,
        })
    }

    fn in_scope(&mut self) -> bool {
        if self.options.states.is_empty() || self.options.states.contains(&UsJurisdiction::Illinois)
        {
            return true;
        }
        let codes: Vec<&str> = self
            .options
            .states
            .iter()
            .map(|state| state.code())
            .collect();
        self.report.note(format!(
            "{codes:?} does not include IL; this adapter covers Illinois only"
        ));
        false
    }

    fn unchanged(&self, row: &MeetRow) -> bool {
        let Some(current) = row.last_refreshed_at.as_deref() else {
            return false;
        };
        self.journal
            .meets
            .get(&row.meet_id.to_string())
            .is_some_and(|recorded| recorded.as_deref() == Some(current))
    }

    async fn walk(&mut self) -> CrawlResult<()> {
        let Some(rows) = requests::meets_index(self.ctx, &mut self.report).await else {
            return Ok(());
        };
        for row in &rows {
            let considered = self.walked.saturating_add(self.skipped_meets);
            if self.options.limit.is_some_and(|limit| considered >= limit) {
                self.report
                    .unfinished
                    .push(format!("{}#meet={}", events_url(row), row.meet_id));
                continue;
            }
            self.meet(row).await?;
        }
        xc::Route {
            ctx: self.ctx,
            report: &mut self.report,
            mapper: &mut self.mapper,
            journal: &mut self.journal,
            resumed: &mut self.resumed_lists,
        }
        .walk()
        .await
    }

    async fn meet(&mut self, row: &MeetRow) -> CrawlResult<()> {
        if self.unchanged(row) {
            self.skipped_meets = self.skipped_meets.saturating_add(1);
            return Ok(());
        }
        let url = events_url(row);
        let Some((envelope, capture)) = requests::events(self.ctx, &mut self.report, &url).await
        else {
            return Ok(());
        };
        let gender = match row.gender.as_str() {
            "Boys" => "M",
            "Girls" => "F",
            _ => "",
        };
        if envelope.meet_id != row.meet_id
            || gender.is_empty()
            || envelope.data.iter().any(|event| event.gender != gender)
        {
            self.refuse(&url, "event index contradicts the requested meet or gender");
            return Ok(());
        }
        if let Err(error) = self.mapper.bind_capture(capture) {
            self.refuse(&url, &error.to_string());
            return Ok(());
        }
        let Some((date, last)) = self.meet_dates(row, &envelope.data, &url) else {
            return Ok(());
        };
        let meet = self.mapper.meet(row, &date, last.as_deref(), &url);
        let (events, complete) = self
            .walk_events(&envelope.data, (row.meet_id, &url), &meet, &date)
            .await;
        if complete {
            self.journal
                .meet(row, &url, (events, self.ctx.performance_as_of));
            self.walked = self.walked.saturating_add(1);
        }
        Ok(())
    }

    fn meet_dates(
        &mut self,
        row: &MeetRow,
        events: &[EventRow],
        url: &str,
    ) -> Option<(String, Option<String>)> {
        let (first, last) = parse::event_date_range(events);
        let valid = first.as_ref().is_some_and(|date| {
            !matches!(
                self.ctx.assess_performance_date(date),
                crate::PerformanceDateAssessment::Unknown
            )
        });
        if !valid {
            self.report.errors = self.report.errors.saturating_add(1);
            self.report.unfinished.push(url.to_string());
            self.report.note(format!(
                "meet {} publishes no valid calendar date; retained source remains owed",
                row.meet_id
            ));
            return None;
        }
        first.map(|date| (date, last))
    }

    fn refuse(&mut self, url: &str, reason: &str) {
        self.report.errors = self.report.errors.saturating_add(1);
        self.report.unfinished.push(url.to_string());
        self.report
            .note(format!("{url}: {reason}; retained unfinished"));
    }

    async fn finish(mut self) -> CrawlResult<AdapterReport> {
        let stats = self.mapper.stats();
        let entries = self.journal.take_pending();
        let counts = self.mapper.store(self.ctx, entries)?;
        let after = self.ctx.fetcher.stats().await;
        self.report.requests = after
            .physical_requests()
            .saturating_sub(self.requests_before);
        self.report.from_cache = after.cache_hits.saturating_sub(self.cache_before);
        self.report.rows =
            u64::try_from(counts.performances).map_err(|_| crate::CrawlError::Invariant {
                detail: "IHSA committed performance counter cannot be represented".into(),
            })?;
        narrate(
            &mut self.report,
            &stats,
            &counts,
            Walked {
                meets: self.walked,
                skipped_meets: self.skipped_meets,
                summaries: self.summaries,
                resumed_lists: self.resumed_lists,
            },
        );
        self.report.finish_frontier();
        Ok(self.report)
    }
}
