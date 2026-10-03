use super::map::{map_contact_row, map_directory_row};
use super::parse::{parse_details, parse_directory, SchoolRow};
use super::{details_url, list_url, LETTERS, SOURCE_ID};
use crate::net::{FetchOptions, FetchOutcome, FetchStats};
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, SchoolId, SourceNamespace,
};
use census_domain::UsJurisdiction;
use census_store::Table;
use serde_json::json;
use std::collections::HashSet;

const JOURNAL: &str = "pa_piaa_schools";

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub letters: Vec<char>,
    pub details_names: Vec<String>,
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
        letters: requested_letters(options),
        details: requested_details(options),
        done: ctx.store.journal_keys(JOURNAL)?,
        report: AdapterReport::new(SOURCE_ID, "schools"),
        processed: 0,
        skipped: 0,
        director_rows: 0,
        with_email: 0,
    };
    if run.covers_requested_state() {
        run.walk_letters().await?;
    } else {
        run.report.note(
            "no requested jurisdiction is Pennsylvania, the only state this association publishes, so nothing was fetched",
        );
    }
    Ok(run.finish(stats_before).await)
}

pub(super) fn requested_letters(options: &Options) -> Vec<char> {
    if options.letters.is_empty() {
        LETTERS.to_vec()
    } else {
        options.letters.clone()
    }
}

pub(super) fn requested_details(options: &Options) -> HashSet<String> {
    options
        .details_names
        .iter()
        .map(|name| normalize_name(name))
        .filter(|name| !name.is_empty())
        .collect()
}

fn letter_key(letter: char) -> String {
    format!("PA:list:{letter}")
}

fn school_key(school_id: &str) -> String {
    format!("PA:school:{school_id}")
}

struct Run<'a> {
    ctx: &'a AdapterContext<'a>,
    options: &'a Options,
    fetch: FetchOptions,
    letters: Vec<char>,
    details: HashSet<String>,
    done: HashSet<String>,
    report: AdapterReport,
    processed: usize,
    skipped: usize,
    director_rows: usize,
    with_email: u64,
}

impl<'a> Run<'a> {
    fn covers_requested_state(&self) -> bool {
        self.options.states.is_empty()
            || self.options.states.contains(&UsJurisdiction::Pennsylvania)
    }

    async fn walk_letters(&mut self) -> CrawlResult<()> {
        let letters = self.letters.clone();
        for (index, letter) in letters.iter().enumerate() {
            if self.options.limit.is_some_and(|limit| index >= limit) {
                return Ok(());
            }
            let key = letter_key(*letter);
            if self.done.contains(&key) {
                self.skipped = self.skipped.saturating_add(1);
                continue;
            }
            self.walk_letter(*letter, key.as_str()).await?;
        }
        Ok(())
    }

    async fn walk_letter(&mut self, letter: char, key: &str) -> CrawlResult<()> {
        let url = list_url(letter);
        let Some(outcome) = self.get(&url).await else {
            return Ok(());
        };
        let rows = match parse_directory(outcome.text().as_str()) {
            Ok(rows) => rows,
            Err(error) => {
                self.fail(format!("letter page {url}: {error}"));
                return Ok(());
            }
        };
        for row in &rows {
            self.process_school(row, url.as_str()).await?;
        }
        self.journal_letter(key, letter, rows.len())
    }

    async fn process_school(&mut self, row: &SchoolRow, list: &str) -> CrawlResult<()> {
        let Some((school, school_id)) =
            map_directory_row(row, list, self.options.observed_on.as_str())
        else {
            self.report.note(format!(
                "directory row {}: no school name, so the row is not a school",
                row.school_id
            ));
            return Ok(());
        };
        let key = school_key(row.school_id.as_str());
        if self.done.contains(&key) {
            self.skipped = self.skipped.saturating_add(1);
            return Ok(());
        }
        let wants_details =
            self.details.is_empty() || self.details.contains(&school.normalized_name);
        let (coaches, read) = if wants_details {
            self.details_coaches(row, &school_id).await?
        } else {
            (Vec::new(), false)
        };
        self.write(key.as_str(), &school, &coaches, row, read)?;
        self.processed = self.processed.saturating_add(1);
        Ok(())
    }

    async fn details_coaches(
        &mut self,
        row: &SchoolRow,
        school_id: &SchoolId,
    ) -> CrawlResult<(Vec<CanonicalCoach>, bool)> {
        let url = details_url(row.school_id.as_str());
        let Some(outcome) = self.get(&url).await else {
            return Ok((Vec::new(), false));
        };
        let page = match parse_details(outcome.text().as_str()) {
            Ok(page) => page,
            Err(error) => {
                self.fail(format!("details page {url}: {error}"));
                return Ok((Vec::new(), false));
            }
        };
        if normalize_name(page.school_name.as_str()) != normalize_name(row.name.as_str()) {
            self.fail(format!(
                "details page {url}: names {}, not the directory's {}",
                page.school_name, row.name
            ));
            return Ok((Vec::new(), false));
        }
        let coaches = page
            .contacts
            .iter()
            .filter_map(|contact| {
                map_contact_row(
                    contact,
                    school_id,
                    url.as_str(),
                    self.options.observed_on.as_str(),
                )
            })
            .collect();
        Ok((coaches, true))
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

    fn journal_letter(&mut self, key: &str, letter: char, rows: usize) -> CrawlResult<()> {
        let mut batch = self.ctx.write_batch();
        batch.journal_done(
            JOURNAL,
            key,
            &json!({
                "state": "PA",
                "letter": letter.to_string(),
                "directory_rows": rows,
                "observed_on": self.options.observed_on,
            }),
        )?;
        batch.commit()?;
        self.done.insert(key.to_string());
        Ok(())
    }

    fn write(
        &mut self,
        key: &str,
        school: &CanonicalSchool,
        coaches: &[CanonicalCoach],
        row: &SchoolRow,
        read: bool,
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
                "state": "PA",
                "school": school.name,
                "school_id": row.school_id,
                "city": school.city.as_deref(),
                "details": read,
                "director_rows": coaches.len(),
                "with_email": with_email,
                "observed_on": self.options.observed_on,
            }),
        )?;
        batch.commit()?;
        self.done.insert(key.to_string());
        self.director_rows = self.director_rows.saturating_add(coaches.len());
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
            "{} school(s) processed ({} already journalled): {} athletic-director row(s), {} with a published address",
            self.processed, self.skipped, self.director_rows, self.with_email
        ));
        self.report.note(
            "the 24 letter pages carry the member schools and their printed address line; each school's details page carries its administrator contacts, and only athletic-director posts are emitted, as `AthleticDirector` rows with no sport",
        );
        self.report
    }
}
