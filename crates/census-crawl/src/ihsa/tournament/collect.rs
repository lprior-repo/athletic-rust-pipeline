use super::journal::Journal;
use super::map::{school_year_of, EventContext, Mapper};
use super::report::{narrate, Walked};
use super::requests::{self, events_url, summary_url};
use super::wire::{EventRow, EventSummary, MeetRow};
use super::{parse, xc};
use crate::ihsa::Options;
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{CanonicalMeet, Sport};
use census_domain::UsJurisdiction;

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

    async fn walk_events(
        &mut self,
        rows: &[EventRow],
        request: (u64, &str),
        meet: &CanonicalMeet,
        date: &str,
    ) -> (usize, bool) {
        let (meet_id, index_url) = request;
        let mut complete = true;
        let index_capture = self.mapper.capture.clone();
        let index_stamp = self.mapper.origin.observed_on.clone();
        for row in rows {
            self.mapper.capture = index_capture.clone();
            self.mapper.origin.observed_on.clone_from(&index_stamp);
            let event = match self.mapper.event(meet, row, index_url) {
                Ok(event) => event,
                Err(error) => {
                    self.refuse_event(index_url, &row.event_id, &error);
                    complete = false;
                    continue;
                }
            };
            self.mapper.count_event(row.has_results);
            if row.has_results {
                let context = self.event_context(row, meet, &event, date);
                if !self.read_summary(row, meet_id, &context).await {
                    complete = false;
                }
            }
        }
        (rows.len(), complete)
    }

    fn refuse(&mut self, url: &str, reason: &str) {
        self.report.errors = self.report.errors.saturating_add(1);
        self.report.unfinished.push(url.to_string());
        self.report
            .note(format!("{url}: {reason}; retained unfinished"));
    }

    fn refuse_event(&mut self, url: &str, event: &str, error: &crate::CrawlError) {
        self.report.errors = self.report.errors.saturating_add(1);
        self.report.note(format!("{url}: event {event}: {error}"));
        self.report.unfinished.push(format!("{url}#event={event}"));
    }

    fn event_context<'b>(
        &self,
        row: &'b EventRow,
        meet: &'b CanonicalMeet,
        event: &'b census_domain::model::CanonicalEvent,
        date: &'b str,
    ) -> EventContext<'b> {
        let date = row
            .scheduled_date
            .as_deref()
            .map(|published| parse::date_part(published).map_or(published, |value| value))
            .map_or(date, |value| value);
        EventContext {
            meet,
            event,
            sport: Sport::OutdoorTrack,
            date,
            school_year: school_year_of(date, self.ctx.school_year),
            performance_as_of: self.ctx.performance_as_of,
        }
    }

    async fn read_summary(
        &mut self,
        row: &EventRow,
        meet_id: u64,
        context: &EventContext<'_>,
    ) -> bool {
        let url = summary_url(&row.event_id);
        let Some((summary, capture)) = requests::summary(self.ctx, &mut self.report, &url).await
        else {
            return false;
        };
        if !summary_matches(&summary, row, meet_id) {
            self.refuse(&url, "summary contradicts the requested event context");
            return false;
        }
        if let Err(error) = self.mapper.bind_capture(capture) {
            self.refuse(&url, &error.to_string());
            return false;
        }
        match self.ctx.assess_performance_date(context.date) {
            crate::context::PerformanceDateAssessment::Future => return true,
            crate::context::PerformanceDateAssessment::Unknown => {
                let error = crate::CrawlError::PerformanceDateUnknown {
                    published: context.date.chars().take(64).collect(),
                    as_of: context.performance_as_of,
                };
                self.refuse(&url, &format!("{error}; original rows retained in capture"));
                return false;
            }
            crate::context::PerformanceDateAssessment::Admitted => {}
        }
        if census_domain::model::SchoolYear::from_date(context.date).is_none() {
            self.refuse(&url, "published date has no supported academic period");
            return false;
        }
        self.mapper.absorb_summary(&summary, context, &url);
        self.summaries = self.summaries.saturating_add(1);
        true
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

fn summary_matches(summary: &EventSummary, row: &EventRow, meet_id: u64) -> bool {
    summary.meet_id == meet_id
        && summary.event_id == row.event_id
        && summary.event_type == row.event_type
        && summary.gender == row.gender
        && summary.class_division == row.class_division
        && summary.event_name == row.event_name
        && summary.round == row.round
        && summary
            .round_label
            .as_deref()
            .is_none_or(|label| Some(label) == parse::round_label(row.round.as_deref()))
        && summary.scheduled_date == row.scheduled_date
        && summary.has_results == row.has_results
}
