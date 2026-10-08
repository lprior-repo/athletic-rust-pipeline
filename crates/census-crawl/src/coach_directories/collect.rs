use super::map::{Capture, CoachCounters};
use super::parse::{DirectoryPage, DirectorySchool};
use super::{Options, MAX_DIRECTORY_PAGES, REGISTERED, SOURCE_ID};
use crate::net::{FetchOptions, FetchOutcome, FetchStats};
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{
    CanonicalCoach, CanonicalSchool, ContactResearchAttempt, SchoolId, SourceNamespace,
};
use census_domain::UsJurisdiction;
use census_store::Table;
use serde_json::json;
use std::collections::{BTreeMap, HashMap, HashSet};

mod directory;
mod frontier;
mod lifecycle;
mod postal;
mod research;
mod schools;
mod setup;

const JOURNAL: &str = "coach_directories_schools_v4";

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let stats_before = ctx.fetcher.stats().await;
    let requested = requested_states(options);
    if requested.is_empty() {
        return Ok(setup::unrequested());
    }
    let mut run = Run::new(ctx, options)?;
    let expected_states = requested.len();
    for (state, association) in requested {
        run.walk_state(state, association).await?;
    }
    run.seal_frontiers()?;
    run.finish_frontier(expected_states);
    Ok(run.finish(stats_before).await)
}

pub(super) fn requested_states(options: &Options) -> Vec<(UsJurisdiction, &'static str)> {
    REGISTERED
        .iter()
        .filter(|(state, _)| options.states.is_empty() || options.states.contains(state))
        .map(|(state, association)| (*state, *association))
        .collect()
}

struct Run<'a> {
    ctx: &'a AdapterContext<'a>,
    options: &'a Options,
    fetch: FetchOptions,
    wanted: HashSet<String>,
    done: HashSet<String>,
    report: AdapterReport,
    processed: usize,
    skipped: usize,
    coach_rows: usize,
    with_email: u64,
    counters: CoachCounters,
    dropped_school_rows: u64,
    researched: HashMap<SchoolId, String>,
    frontiers: BTreeMap<UsJurisdiction, ContactResearchAttempt>,
    incomplete_states: HashSet<UsJurisdiction>,
    drained_states: usize,
    seen_names: HashSet<String>,
}

impl<'a> Run<'a> {
    async fn get(&mut self, url: &str) -> Option<FetchOutcome> {
        match self.ctx.fetcher.get(url, &self.fetch).await {
            Ok(outcome) => Some(outcome),
            Err(error) => {
                self.report.disposition =
                    lifecycle::failure_disposition(super::research_failure::fetch(&error));
                self.report
                    .unfinished
                    .push(format!("directory fetch remains unresolved: {url}"));
                self.fail(format!("fetch {url}: {error}"));
                None
            }
        }
    }

    fn fail(&mut self, message: String) {
        self.report.errors = self.report.errors.saturating_add(1);
        self.report.note(message);
    }

    fn reject(&mut self, message: String) {
        self.report.reject(message);
    }

    fn school_batch(
        &self,
        school: &CanonicalSchool,
    ) -> CrawlResult<crate::recording::RowBatch<'_>> {
        let mut batch = self.ctx.write_batch();
        batch.append_many(Table::Schools, std::slice::from_ref(school))?;
        batch.append_many(
            Table::SourceObservations,
            self.ctx
                .school_observation(&SourceNamespace::association_school(SOURCE_ID), school)
                .as_slice(),
        )?;
        Ok(batch)
    }

    fn write(
        &mut self,
        key: &str,
        school: &CanonicalSchool,
        coaches: &[CanonicalCoach],
        short_code: &str,
    ) -> CrawlResult<()> {
        let mut batch = self.school_batch(school)?;
        let with_email = coaches
            .iter()
            .filter(|coach| coach.has_published_email())
            .count();
        batch.append_many(Table::Coaches, coaches)?;
        batch.journal_done(
            JOURNAL,
            key,
            &json!({
                "state": school.state.map(|state| state.code().to_string()),
                "school": school.name,
                "short_code": short_code,
                "coach_rows": coaches.len(),
                "with_email": with_email,
                "observed_on": self.ctx.observed_on,
            }),
        )?;
        batch.commit()?;
        self.done.insert(key.to_string());
        self.coach_rows = self.coach_rows.saturating_add(coaches.len());
        self.with_email = self
            .with_email
            .saturating_add(u64::try_from(with_email).map_or(u64::MAX, |value| value));
        Ok(())
    }

    async fn finish(mut self, stats_before: FetchStats) -> AdapterReport {
        let stats_after = self.ctx.fetcher.stats().await;
        self.report.requests = stats_after
            .physical_requests()
            .saturating_sub(stats_before.physical_requests());
        self.report.from_cache = stats_after
            .cache_hits
            .saturating_sub(stats_before.cache_hits);
        self.report.rows = u64::try_from(self.processed).map_or(u64::MAX, |value| value);
        self.report.with_email = self.with_email;
        self.report.note(format!(
            "{} school(s) processed ({} already journalled): {} coach row(s), {} with a published email address",
            self.processed, self.skipped, self.coach_rows, self.with_email
        ));
        let breakdown = self.counters.breakdown();
        let sub_varsity = if breakdown.is_empty() {
            "none".to_string()
        } else {
            breakdown
        };
        self.report.note(format!(
            "directory pages carry school identity and the association's competition levels; summaries carry the school's teams and staff, and the lane emits the prototype's varsity scope (ADR-016 S12/S13): {} coach row(s) dropped for a non-varsity level ({sub_varsity}), {} dropped for a vendor address, {} dropped as a post rather than a person, {} directory row(s) dropped for an empty or vendor school name",
            self.counters.dropped_total(),
            self.counters.dropped_vendor,
            self.counters.dropped_person,
            self.dropped_school_rows,
        ));
        self.report
    }
}
