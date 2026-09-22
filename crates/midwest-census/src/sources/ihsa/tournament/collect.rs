//! The walk: the meets index and its own change signal, one summary per event that publishes
//! results, then the archive's cross-country state-finalist lists.
//!
//! The request budget is the index's shape, not a choice: `GET /v1/track-field/meets` answers two
//! state-final meets (boys, girls) per season, each meet's events index answers one row per
//! class x round x event instance (97 for the captured boys meet), and an event's summary is read
//! only when its index row says `hasResults`. A season therefore costs a measured ~200 requests, and
//! a re-run of an unchanged season costs one: the index publishes `LastRefreshedAt` per meet and the
//! journal keeps it.

use super::journal::Journal;
use super::map::{school_year_of, EventContext, Mapper};
use super::report::{narrate, Walked};
use super::requests::{self, events_url, summary_url};
use super::wire::{EventRow, MeetRow};
use super::{parse, xc};
use crate::sources::ihsa::Options;
use crate::sources::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{CanonicalMeet, Sport};
use census_domain::UsJurisdiction;

/// The adapter this walk files evidence under (its registry slug).
const SOURCE: &str = "ihsa";

/// Collect the IHSA's state-final track & field results and cross-country state-finalist lists.
///
/// `limit` bounds the meets walked, which is the unit the meets index publishes; the qualifier lists
/// are read whatever the limit, since they are six requests and not per-meet work.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let before = ctx.fetcher.stats().await;
    let mut run = Run::new(ctx, options)?;
    run.requests_before = before.requests;
    run.cache_before = before.cache_hits;
    if !run.in_scope() {
        return Ok(run.report);
    }
    run.walk().await?;
    run.finish().await
}

/// One run's state: the store handle, the mapping sinks, the journal and the tallies.
///
/// Tallies saturate: they feed diagnostics only, so an impossible overflow floors instead of
/// panicking or wrapping silently.
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
    /// Open a run: the journal's resume points and the store's schools.
    fn new(ctx: &'a AdapterContext<'a>, options: &'a Options) -> CrawlResult<Self> {
        Ok(Self {
            ctx,
            options,
            mapper: Mapper::load(ctx, SOURCE, &options.observed_on)?,
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

    /// Whether the operator's jurisdiction filter admits Illinois.
    ///
    /// A run scoped to other states spends no request here; the note says why the report is empty.
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

    /// Whether the index's own change signal says this meet is already read.
    ///
    /// A meet the index publishes no `LastRefreshedAt` for is re-read every run: without a signal the
    /// only honest answer is that its state is unknown.
    fn unchanged(&self, row: &MeetRow) -> bool {
        let Some(current) = row.last_refreshed_at.as_deref() else {
            return false;
        };
        self.journal
            .meets
            .get(&row.meet_id.to_string())
            .is_some_and(|recorded| recorded.as_deref() == Some(current))
    }

    /// Walk the meets index, then the cross-country lists.
    async fn walk(&mut self) -> CrawlResult<()> {
        let Some(rows) = requests::meets_index(self.ctx, &mut self.report).await else {
            return Ok(());
        };
        for row in &rows {
            // The limit bounds the index rows this run considers, not the meets it happens to walk:
            // a resumed run skips the rows it already has, and counting only walked meets would walk
            // the next row — a meet another run owns — to fill the quota.
            let considered = self.walked.saturating_add(self.skipped_meets);
            if self.options.limit.is_some_and(|limit| considered >= limit) {
                break;
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

    /// Walk one meet: its events index, then one summary per event that publishes results.
    ///
    /// The meet's journal entry is written only when every request it needs landed, so a run that
    /// loses one summary leaves the whole meet for the next run rather than recording a half-read
    /// state the change signal would then skip.
    async fn meet(&mut self, row: &MeetRow) -> CrawlResult<()> {
        if self.unchanged(row) {
            self.skipped_meets = self.skipped_meets.saturating_add(1);
            return Ok(());
        }
        let url = events_url(row);
        let Some(envelope) = requests::events(self.ctx, &mut self.report, &url).await else {
            return Ok(());
        };
        let (first, last) = parse::event_date_range(&envelope.data);
        let Some(date) = first else {
            self.report.note(format!(
                "meet {}: no event publishes a date to file the meet under; left for the next run",
                row.meet_id
            ));
            return Ok(());
        };
        let meet = self.mapper.meet(row, &date, last.as_deref(), &url);
        let (events, complete) = self.walk_events(&envelope.data, &meet, &date, &url).await;
        if complete {
            self.journal.meet(self.ctx, row, &url, events)?;
            self.walked = self.walked.saturating_add(1);
        }
        Ok(())
    }

    /// Mint one event row per index row, then read and map the summary of every one with results.
    ///
    /// Returns the number of index rows minted and whether every summary request landed.
    async fn walk_events(
        &mut self,
        rows: &[EventRow],
        meet: &CanonicalMeet,
        date: &str,
        index_url: &str,
    ) -> (usize, bool) {
        let school_year = school_year_of(date, self.ctx.school_year);
        let mut complete = true;
        for row in rows {
            let event = self.mapper.event(meet, row, index_url);
            self.mapper.count_event(row.has_results);
            if !row.has_results {
                continue;
            }
            let url = summary_url(&row.event_id);
            let Some(summary) = requests::summary(self.ctx, &mut self.report, &url).await else {
                complete = false;
                continue;
            };
            let context = EventContext {
                meet,
                event: &event,
                sport: Sport::OutdoorTrack,
                date: row
                    .scheduled_date
                    .as_deref()
                    .and_then(parse::date_part)
                    .unwrap_or(date),
                school_year,
            };
            self.mapper.absorb_summary(&summary, &context, &url);
            self.summaries = self.summaries.saturating_add(1);
        }
        (rows.len(), complete)
    }

    /// Append what the run accumulated and report it.
    async fn finish(mut self) -> CrawlResult<AdapterReport> {
        let stats = self.mapper.stats();
        let counts = self.mapper.store(self.ctx)?;
        let after = self.ctx.fetcher.stats().await;
        self.report.requests = after.requests.saturating_sub(self.requests_before);
        self.report.from_cache = after.cache_hits.saturating_sub(self.cache_before);
        self.report.rows = stats.performances;
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
        Ok(self.report)
    }
}
