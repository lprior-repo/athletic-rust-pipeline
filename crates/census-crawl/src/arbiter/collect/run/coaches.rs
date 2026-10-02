use super::recovery::Recovery;
use super::{page_read_short, response_text, Run};
use crate::arbiter::map::map_coach_row;
use crate::arbiter::parse::{parse_coach_outcomes, CoachRow, OrgSchool, Page};
use crate::arbiter::{MAX_PAGES, PAGE_SIZE};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{CanonicalCoach, CanonicalSchool};

pub(super) struct CoachAcquisition {
    pub(super) coaches: Vec<CanonicalCoach>,
    pub(super) completion: CrawlResult<()>,
}

impl CoachAcquisition {
    fn incomplete(coaches: Vec<CanonicalCoach>, error: CrawlError) -> Self {
        Self {
            coaches,
            completion: Err(error),
        }
    }
}

impl Run<'_> {
    pub(super) fn finish_school(
        &mut self,
        org: &str,
        row: &OrgSchool,
        school: &CanonicalSchool,
        key: Option<String>,
        acquisition: CoachAcquisition,
        recovery: &Recovery,
    ) -> CrawlResult<()> {
        match (acquisition.completion, key) {
            (Ok(()), Some(key)) => self.write(org, row, school, &acquisition.coaches, &key)?,
            (Ok(()), None) => {
                return Err(CrawlError::Invariant {
                    detail: "completed Arbiter acquisition has no source owner".to_string(),
                });
            }
            (Err(error), key) => {
                let mut batch = self.persist_facts(school, &acquisition.coaches)?;
                if let Some(key) = key {
                    recovery.persist(&mut batch, &school.id, &key, &error)?;
                }
                batch.commit()?;
                self.tally.coaches = self.tally.coaches.saturating_add(acquisition.coaches.len());
                self.tally.fail(format!(
                    "{} coaches for {}: {error}; completion owed",
                    failure_kind(&error),
                    school.name,
                ));
            }
        }
        self.tally.schools = self.tally.schools.saturating_add(1);
        Ok(())
    }

    pub(super) async fn collect_coaches(
        &self,
        host: &str,
        org: &str,
        public_id: u64,
        school: &CanonicalSchool,
        recovery: &mut Recovery,
    ) -> CoachAcquisition {
        let mut coaches = Vec::new();
        let mut completion = Ok(());
        for page in 1..=MAX_PAGES {
            let url = format!(
                "{host}/api/v2/legacy/public/{org}/coaches?filter.EntityId={public_id}&&pageSize={PAGE_SIZE}&pageNumber={page}"
            );
            let (parsed, rows_on_page) = match self.coach_page(&url, page, recovery).await {
                Ok(parsed) => parsed,
                Err(error) => return CoachAcquisition::incomplete(coaches, error),
            };
            completion = completion.and(retain_coaches(
                &mut coaches,
                parsed.rows,
                school,
                &url,
                &self.options.observed_on,
            ));
            if rows_on_page < PAGE_SIZE {
                return CoachAcquisition {
                    coaches,
                    completion: completion.and(short_page_completion(
                        page,
                        rows_on_page,
                        parsed.total,
                        url,
                        &school.name,
                    )),
                };
            }
        }
        CoachAcquisition::incomplete(
            coaches,
            CrawlError::Schema {
                url: format!(
                    "{host}/api/v2/legacy/public/{org}/coaches?filter.EntityId={public_id}"
                ),
                detail: format!(
                    "coaches for {} have more pages than the {MAX_PAGES}-page walk reads",
                    school.name,
                ),
            },
        )
    }

    async fn coach_page(
        &self,
        url: &str,
        page: u64,
        recovery: &mut Recovery,
    ) -> CrawlResult<(Page<CrawlResult<CoachRow>>, u64)> {
        let outcome = recovery.fetch(self, url).await?;
        let parsed = match response_text(&outcome, "coach")
            .and_then(|body| parse_coach_outcomes(body, url))
        {
            Ok(parsed) => parsed,
            Err(error) => {
                recovery.remember(url, &outcome)?;
                return Err(error);
            }
        };
        let count = u64::try_from(parsed.rows.len()).map_err(|_| CrawlError::Arithmetic {
            detail: "Arbiter coach page row count exceeds u64".to_string(),
        })?;
        let short = count < PAGE_SIZE && page_read_short(page, count, parsed.total);
        let bounded = (page == 1 && parsed.total > MAX_PAGES.saturating_mul(PAGE_SIZE))
            || (page == MAX_PAGES && count >= PAGE_SIZE);
        if short || bounded || parsed.rows.iter().any(Result::is_err) {
            recovery.remember(url, &outcome)?;
        }
        Ok((parsed, count))
    }
}

fn retain_coaches(
    coaches: &mut Vec<CanonicalCoach>,
    rows: Vec<CrawlResult<CoachRow>>,
    school: &CanonicalSchool,
    url: &str,
    observed_on: &str,
) -> CrawlResult<()> {
    let mut completion = Ok(());
    for row in rows {
        match row {
            Ok(row) => {
                if let Some(coach) = map_coach_row(&row, &school.id, url, observed_on) {
                    coaches.push(coach);
                }
            }
            Err(error) => completion = completion.and(Err(error)),
        }
    }
    completion
}

fn short_page_completion(
    page: u64,
    rows_on_page: u64,
    total: u64,
    url: String,
    school_name: &str,
) -> CrawlResult<()> {
    if !page_read_short(page, rows_on_page, total) {
        return Ok(());
    }
    Err(CrawlError::Schema {
        url,
        detail: format!(
            "coaches for {school_name}: page {page} returned {rows_on_page} of \
             {PAGE_SIZE} rows while the response totals {total}"
        ),
    })
}

pub(super) fn failure_kind(error: &CrawlError) -> &'static str {
    match error {
        CrawlError::Fetch(error) if error.retryable() => "retryable",
        CrawlError::Fetch(
            crate::net::FetchError::Http {
                status: 401 | 403, ..
            }
            | crate::net::FetchError::Policy { .. }
            | crate::net::FetchError::BrowserLane {
                retryable: false, ..
            },
        ) => "source_refused",
        _ => "partial",
    }
}
