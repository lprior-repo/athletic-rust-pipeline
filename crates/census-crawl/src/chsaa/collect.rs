use super::map::{map_coach_row, map_directory_row};
use super::parse::{parse_directory, parse_school_page, MemberSchool};
use super::{nonempty, school_page_url, DIRECTORY_URL, SOURCE_ID};
use crate::net::{FetchOptions, FetchOutcome, FetchStats};
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{normalize_name, CanonicalCoach, CanonicalSchool, SourceNamespace};
use census_domain::UsJurisdiction;
use census_store::Table;
use serde_json::json;
use std::collections::HashSet;

const JOURNAL: &str = "chsaa_schools";

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let stats_before = ctx.fetcher.stats().await;
    let mut run = Run {
        ctx,
        options,
        fetch: FetchOptions {
            refresh: options.refresh || ctx.refresh,
            allow_not_found: false,
            headers: Vec::new(),
        },
        wanted: wanted_names(options),
        done: ctx.store.journal_keys(JOURNAL)?,
        report: AdapterReport::new(SOURCE_ID, "schools"),
        processed: 0,
        skipped: 0,
        coach_rows: 0,
        with_email: 0,
    };
    if run.covers_requested_state() {
        run.walk_directory().await?;
    } else {
        run.report.note(
            "no requested jurisdiction is Colorado, the only state this association publishes, so nothing was fetched",
        );
    }
    Ok(run.finish(stats_before).await)
}

pub(super) fn wanted_names(options: &Options) -> HashSet<String> {
    options
        .school_names
        .iter()
        .map(|name| normalize_name(name))
        .filter(|name| !name.is_empty())
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
}

impl<'a> Run<'a> {
    fn covers_requested_state(&self) -> bool {
        self.options.states.is_empty() || self.options.states.contains(&UsJurisdiction::Colorado)
    }

    async fn walk_directory(&mut self) -> CrawlResult<()> {
        let Some(outcome) = self.get(DIRECTORY_URL).await else {
            return Ok(());
        };
        let schools = match parse_directory(&outcome.text()) {
            Ok(schools) => schools,
            Err(error) => {
                self.fail(format!("directory {DIRECTORY_URL}: {error}"));
                return Ok(());
            }
        };
        for school in &schools {
            if self
                .options
                .limit
                .is_some_and(|limit| self.processed >= limit)
            {
                return Ok(());
            }
            self.process_school(school).await?;
        }
        Ok(())
    }

    async fn process_school(&mut self, row: &MemberSchool) -> CrawlResult<()> {
        let Some(slug) = row.slug.as_deref().and_then(nonempty) else {
            self.report
                .note("directory row without a school slug, so its page cannot be named");
            return Ok(());
        };
        let key = format!("CO:{slug}");
        if self.done.contains(&key) {
            self.skipped = self.skipped.saturating_add(1);
            return Ok(());
        }
        let Some((school, school_id)) =
            map_directory_row(row, DIRECTORY_URL, self.options.observed_on.as_str())
        else {
            self.report.note(format!(
                "directory row {slug}: no school name, so the row is not a school"
            ));
            return Ok(());
        };
        if !self.wanted.is_empty() && !self.wanted.contains(&school.normalized_name) {
            return Ok(());
        }
        let url = school_page_url(slug.as_str());
        let coaches: Vec<CanonicalCoach> = match self.get(&url).await {
            Some(outcome) => match parse_school_page(&outcome.text()) {
                Ok(rows) => rows
                    .iter()
                    .filter_map(|row| {
                        map_coach_row(row, &school_id, &url, self.options.observed_on.as_str())
                    })
                    .collect(),
                Err(error) => {
                    self.fail(format!("school page {url}: {error}"));
                    return Ok(());
                }
            },
            None => return Ok(()),
        };
        self.write(&key, &school, &coaches, slug.as_str())?;
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

    fn write(
        &mut self,
        key: &str,
        school: &CanonicalSchool,
        coaches: &[CanonicalCoach],
        slug: &str,
    ) -> CrawlResult<()> {
        let namespace = SourceNamespace::association_school(SOURCE_ID);
        let mut batch = self.ctx.write_batch();
        batch.append_many(Table::Schools, std::slice::from_ref(school))?;
        batch.append_many(
            Table::SourceObservations,
            self.ctx.school_observation(&namespace, school).as_slice(),
        )?;
        batch.append_many(Table::Coaches, coaches)?;
        let with_email = coaches
            .iter()
            .filter(|coach| coach.has_published_email())
            .count();
        batch.journal_done(
            JOURNAL,
            key,
            &json!({
                "state": "CO",
                "school": school.name,
                "slug": slug,
                "coach_rows": coaches.len(),
                "with_email": with_email,
                "observed_on": self.options.observed_on,
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
            "{} school(s) processed ({} already journalled): {} coach row(s), {} with a published address",
            self.processed, self.skipped, self.coach_rows, self.with_email
        ));
        self.report.note(
            "the member directory carries school identity and address; each school page carries its activity roster, and every track, cross-country, head- and assistant-coach row it publishes is emitted at every level (the census model stores no coach level, so sub-varsity rows are kept rather than filtered)",
        );
        self.report
    }
}
