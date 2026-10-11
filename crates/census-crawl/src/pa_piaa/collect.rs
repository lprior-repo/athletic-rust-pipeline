use super::map::{map_contact_row, map_directory_row};
use super::parse::{parse_details, parse_directory, SchoolRow};
use super::{details_url, list_url, LETTERS, SOURCE_ID};
use crate::net::{FetchOptions, FetchOutcome};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{normalize_name, CanonicalCoach, CanonicalSchool, SchoolId};
use census_domain::UsJurisdiction;
use futures::{stream, StreamExt, TryStreamExt};

use census_domain::model::ContactResearchOutcome as Outcome;
mod write;
const JOURNAL: &str = "pa_piaa_projection_v2";

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub letters: Vec<char>,
    pub details_names: Vec<String>,
}

#[tracing::instrument(skip(ctx, options))]
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let before = ctx.fetcher.stats().await;
    let mut run = Run {
        ctx,
        options,
        fetch: FetchOptions {
            refresh: options.refresh || ctx.refresh,
            ..ctx.fetch_options()
        },
        report: AdapterReport::new(SOURCE_ID, "schools"),
    };
    if options.states.is_empty() || options.states.contains(&UsJurisdiction::Pennsylvania) {
        run = run.walk_letters().await?;
    }
    let after = ctx.fetcher.stats().await;
    run.report.requests = after
        .physical_requests()
        .checked_sub(before.physical_requests())
        .ok_or_else(counter_error)?;
    run.report.from_cache = after
        .cache_hits
        .checked_sub(before.cache_hits)
        .ok_or_else(counter_error)?;
    Ok(run.report)
}

pub(super) fn requested_letters(options: &Options) -> Vec<char> {
    if options.letters.is_empty() {
        LETTERS.to_vec()
    } else {
        options.letters.clone()
    }
}

fn counter_error() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "PIAA counter overflow or reversal".to_string(),
    }
}

struct Run<'a> {
    ctx: &'a AdapterContext<'a>,
    options: &'a Options,
    fetch: FetchOptions,
    report: AdapterReport,
}

struct Projection<'a> {
    school: CanonicalSchool,
    coaches: Vec<CanonicalCoach>,
    list: &'a FetchOutcome,
    detail: Option<FetchOutcome>,
    detail_outcome: Outcome,
}

impl Run<'_> {
    #[tracing::instrument(skip(self))]
    async fn walk_letters(self) -> CrawlResult<Self> {
        let letters = requested_letters(self.options);
        let mut run = stream::iter(letters.into_iter().enumerate())
            .map(Ok::<_, CrawlError>)
            .try_fold(self, |mut run, (index, letter)| async move {
                if run.options.limit.is_some_and(|limit| index >= limit) {
                    run.owe(&list_url(letter))?;
                } else {
                    run.walk_letter(letter).await?;
                }
                Ok(run)
            })
            .await?;
        run.report.finish_frontier();
        Ok(run)
    }

    #[tracing::instrument(skip(self))]
    async fn walk_letter(&mut self, letter: char) -> CrawlResult<()> {
        let url = list_url(letter);
        let (capture, state) = self.get(&url).await?;
        let Some(capture) = capture.filter(|_| state.is_terminal()) else {
            return Ok(());
        };
        let rows = match std::str::from_utf8(&capture.body)
            .map_err(|error| error.to_string())
            .and_then(|text| parse_directory(text).map_err(|error| error.to_string()))
        {
            Ok(rows) => rows,
            Err(error) => {
                self.fail(&url, error)?;
                return Ok(());
            }
        };
        let capture = &capture;
        stream::iter(rows.iter())
            .map(Ok::<_, CrawlError>)
            .try_fold(self, |run, row| async move {
                run.process_school(row, capture).await?;
                Ok(run)
            })
            .await?;
        Ok(())
    }

    #[tracing::instrument(skip(self, row, list))]
    async fn process_school(&mut self, row: &SchoolRow, list: &FetchOutcome) -> CrawlResult<()> {
        let url = details_url(&row.school_id);
        let Some((school, id)) = map_directory_row(row, &list.url, &list.fetched_at) else {
            self.fail(&url, "directory row has no school name".to_string())?;
            return Ok(());
        };
        let wants_details = self.options.details_names.is_empty()
            || self
                .options
                .details_names
                .iter()
                .any(|wanted| normalize_name(wanted) == school.normalized_name);
        let (coaches, detail, detail_outcome) = if wants_details {
            self.details_coaches(row, &id).await?
        } else {
            self.owe(&url)?;
            (Vec::new(), None, Outcome::Unattempted)
        };
        if !coaches.is_empty() {
            self.owe(&url)?;
            self.report
                .note("appointment season is not published; tenure remains unknown");
        }
        self.write(Projection {
            school,
            coaches,
            list,
            detail,
            detail_outcome,
        })
    }

    #[tracing::instrument(skip(self, row, id))]
    async fn details_coaches(
        &mut self,
        row: &SchoolRow,
        id: &SchoolId,
    ) -> CrawlResult<(Vec<CanonicalCoach>, Option<FetchOutcome>, Outcome)> {
        let url = details_url(&row.school_id);
        let (capture, outcome) = self.get(&url).await?;
        let Some(capture) = capture else {
            return Ok((Vec::new(), None, outcome));
        };
        if !outcome.is_terminal() {
            return Ok((Vec::new(), Some(capture), outcome));
        }
        let page = match std::str::from_utf8(&capture.body)
            .map_err(|error| error.to_string())
            .and_then(|text| parse_details(text).map_err(|error| error.to_string()))
        {
            Ok(page) => page,
            Err(error) => {
                self.fail(&url, error)?;
                return Ok((Vec::new(), Some(capture), Outcome::Failed));
            }
        };
        if normalize_name(&page.school_name) != normalize_name(&row.name) {
            self.fail(&url, "details owner differs from directory".to_string())?;
            return Ok((Vec::new(), Some(capture), Outcome::Ambiguous));
        }
        let mut coaches = Vec::new();
        coaches
            .try_reserve(page.contacts.len())
            .map_err(|_| CrawlError::Resource {
                resource: "PIAA contacts",
                requested: page.contacts.len(),
                limit: crate::net::MAX_BODY_BYTES,
            })?;
        let mut outcome = Outcome::CompletedEmpty;
        page.contacts.iter().for_each(|contact| {
            if let Some(coach) = map_contact_row(contact, id, &capture.url, &capture.fetched_at) {
                coaches.push(coach);
            } else {
                outcome = Outcome::Partial;
            }
        });
        if !outcome.is_terminal() {
            self.owe(&url)?;
            self.report
                .note("published contact rows without an admissible person remain unresolved");
        }
        Ok((coaches, Some(capture), outcome))
    }

    #[tracing::instrument(skip(self))]
    async fn get(&mut self, url: &str) -> CrawlResult<(Option<FetchOutcome>, Outcome)> {
        match self.ctx.fetcher.get(url, &self.fetch).await {
            Ok(capture) if capture.status == 200 => Ok((Some(capture), Outcome::CompletedEmpty)),
            Ok(capture) => {
                self.fail(url, format!("HTTP {}", capture.status))?;
                let state = crate::coach_directories::contact_status_failure(capture.status);
                Ok((Some(capture), state))
            }
            Err(error) => {
                self.fail(url, error.to_string())?;
                Ok((
                    None,
                    crate::coach_directories::contact_fetch_failure(&error),
                ))
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
                resource: "PIAA unfinished locators",
                requested: 1,
                limit: crate::net::MAX_BODY_BYTES,
            })?;
        self.report.unfinished.push(url.to_string());
        self.report.disposition = crate::CollectionDisposition::Partial;
        Ok(())
    }

    fn fail(&mut self, url: &str, detail: String) -> CrawlResult<()> {
        self.report.errors = self
            .report
            .errors
            .checked_add(1)
            .ok_or_else(counter_error)?;
        self.owe(url)?;
        if self.report.errors <= 5 {
            self.report.note(
                format!("{url}: {detail}")
                    .chars()
                    .take(4096)
                    .collect::<String>(),
            );
        }
        Ok(())
    }
}
