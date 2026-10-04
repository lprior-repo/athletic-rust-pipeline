use super::map::{retain_directory_postal, Capture, CoachCounters};
use super::parse::{parse_directory, DirectoryPage, DirectorySchool};
use super::{directory_page_url, Options, MAX_DIRECTORY_PAGES, REGISTERED, SOURCE_ID};
use crate::net::{FetchOptions, FetchOutcome, FetchStats};
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{normalize_name, CanonicalCoach, CanonicalSchool, SourceNamespace};
use census_domain::UsJurisdiction;
use census_store::Table;
use serde_json::json;
use std::collections::HashSet;

mod postal;

const JOURNAL: &str = "coach_directories_schools_v3";

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let stats_before = ctx.fetcher.stats().await;
    let requested = requested_states(options);
    if requested.is_empty() {
        let mut report = AdapterReport::new(SOURCE_ID, "schools");
        report.note(
            "no requested jurisdiction is one of the 15 associations that publish staff on this platform, so nothing was fetched",
        );
        return Ok(report);
    }
    let wanted: HashSet<String> = options
        .school_names
        .iter()
        .map(|name| normalize_name(name))
        .filter(|name| !name.is_empty())
        .collect();
    let mut run = Run {
        ctx,
        options,
        fetch: FetchOptions {
            refresh: options.refresh || ctx.refresh,
            allow_not_found: false,
            headers: Vec::new(),
        },
        wanted,
        done: ctx.store.journal_keys(JOURNAL)?,
        report: AdapterReport::new(SOURCE_ID, "schools"),
        processed: 0,
        skipped: 0,
        coach_rows: 0,
        with_email: 0,
        counters: CoachCounters::default(),
        dropped_school_rows: 0,
    };
    for (state, association) in requested {
        run.walk_state(state, association).await?;
    }
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
}

impl<'a> Run<'a> {
    async fn walk_state(&mut self, state: UsJurisdiction, association: &str) -> CrawlResult<()> {
        let mut page = 1usize;
        let mut total_pages = 1usize;
        let mut rows_read = 0usize;
        let mut declared_rows = 0usize;
        while page <= total_pages && page <= MAX_DIRECTORY_PAGES {
            let url = directory_page_url(association, page);
            let Some(outcome) = self.get(&url).await else {
                return Ok(());
            };
            let parsed = match parse_directory(&outcome.body) {
                Ok(parsed) => parsed,
                Err(error) => {
                    self.fail(format!("directory page {url}: {error}"));
                    return Ok(());
                }
            };
            if !self.directory_page_matches(&parsed, page, &url) {
                return Ok(());
            }
            total_pages = total_pages.max(parsed.total_pages.max(1));
            rows_read = rows_read.saturating_add(parsed.results.len());
            declared_rows = declared_rows.max(parsed.total_results);
            let capture = Capture {
                url: &outcome.url,
                observed_on: &outcome.fetched_at,
                sha256: &outcome.content_digest,
            };
            for row in &parsed.results {
                if self
                    .options
                    .limit
                    .is_some_and(|limit| self.processed >= limit)
                {
                    return Ok(());
                }
                self.process_school(state, association, capture, row)
                    .await?;
            }
            page = page.saturating_add(1);
        }
        if total_pages > MAX_DIRECTORY_PAGES {
            self.fail(format!(
                "{} has more directory pages than the {MAX_DIRECTORY_PAGES}-page walk reads",
                state.code()
            ));
        }
        if rows_read < declared_rows {
            self.fail(format!(
                "{} directory walk stopped short: read {rows_read} of {declared_rows} published rows",
                state.code()
            ));
        }
        Ok(())
    }

    async fn process_school(
        &mut self,
        state: UsJurisdiction,
        association: &str,
        capture: Capture<'_>,
        row: &DirectorySchool,
    ) -> CrawlResult<()> {
        let Some(short_code) = self.directory_owner(state, capture, row) else {
            return Ok(());
        };
        let key = format!("{}:{short_code}", state.code());
        if self.done.contains(&key) && !self.fetch.refresh {
            self.skipped = self.skipped.saturating_add(1);
            return Ok(());
        }
        let Some((mut school, school_id)) =
            self.admit_directory_school(state, association, capture, row)?
        else {
            return Ok(());
        };
        if !self.wanted.is_empty() && !self.wanted.contains(&school.normalized_name) {
            return Ok(());
        }
        let postal_complete = match retain_directory_postal(&mut school, row, capture) {
            Ok(()) => true,
            Err(review) => {
                self.fail(format!("directory postal review {}: {review}", capture.url));
                false
            }
        };
        let Some(mapped) = self
            .fetch_and_process_summary(row, &short_code, &mut school, &school_id)
            .await
        else {
            self.school_batch(&school)?.commit()?;
            return Ok(());
        };
        let emission = mapped.emission;
        if postal_complete && mapped.postal_review.is_none() {
            self.write(&key, &school, &emission.coaches, &short_code)?;
        } else {
            self.retain_incomplete_summary(&school, &emission)?;
        }
        self.counters.absorb(&emission.counters);
        self.processed = self.processed.saturating_add(1);
        Ok(())
    }

    async fn get(&mut self, url: &str) -> Option<FetchOutcome> {
        match self.ctx.fetcher.get(url, &self.fetch).await {
            Ok(outcome) => Some(outcome),
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
        self.report.requests = stats_after.requests.saturating_sub(stats_before.requests);
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
