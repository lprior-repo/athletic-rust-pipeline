use super::super::map::{map_org_school, map_primary_contact};
use super::super::parse::{parse_org_schools, OrgSchool};
use super::super::{HOST, MAX_PAGES, PAGE_SIZE, SOURCE_ID};
use crate::net::{FetchOptions, FetchOutcome};
use crate::{AdapterContext, CrawlError, CrawlResult};
use census_domain::model::{CanonicalCoach, CanonicalSchool, SourceNamespace};
use census_domain::UsJurisdiction;
use census_store::Table;
use serde_json::json;
use std::collections::HashSet;

mod coaches;
mod recovery;

#[cfg(test)]
mod tests;

pub(in crate::arbiter) const JOURNAL: &str = "arbiter_owned_coaches_v2";
const MAX_NOTES: usize = 32;

#[derive(Default)]
pub(in crate::arbiter) struct Tally {
    pub(in crate::arbiter) schools: usize,
    pub(in crate::arbiter) coaches: usize,
    pub(in crate::arbiter) skipped: usize,
    pub(in crate::arbiter) errors: usize,
    pub(in crate::arbiter) notes: Vec<String>,
}

impl Tally {
    fn fail(&mut self, message: String) {
        self.errors = self.errors.saturating_add(1);
        if self.notes.len() < MAX_NOTES {
            self.notes.push(message);
        }
    }
}

pub(in crate::arbiter) struct Run<'a> {
    pub(in crate::arbiter) ctx: &'a AdapterContext<'a>,
    pub(in crate::arbiter) options: &'a super::Options,
    pub(in crate::arbiter) fetch: FetchOptions,
    pub(in crate::arbiter) done: HashSet<String>,
    pub(in crate::arbiter) tally: Tally,
}

impl Run<'_> {
    pub(in crate::arbiter) async fn walk(
        &mut self,
        state: UsJurisdiction,
        org: &str,
    ) -> CrawlResult<()> {
        let base_url = format!("{HOST}/api/v2/organization/public/{org}/children");
        let mut page = 1u64;
        let mut ended = false;
        while page <= MAX_PAGES {
            if self.at_limit() {
                break;
            }
            let url = format!("{base_url}?&pageSize={PAGE_SIZE}&pageNumber={page}");
            let Some(outcome) = self.fetch_page(&url).await else {
                ended = true;
                break;
            };
            let parsed = match response_text(&outcome, "member")
                .and_then(|body| parse_org_schools(body, &url))
            {
                Ok(parsed) => parsed,
                Err(error) => {
                    self.tally
                        .fail(format!("schools page {page} for {}: {error}", state.code()));
                    ended = true;
                    break;
                }
            };
            let rows_on_page =
                u64::try_from(parsed.rows.len()).map_err(|_| CrawlError::Arithmetic {
                    detail: "Arbiter member page row count exceeds u64".to_string(),
                })?;
            let last_page = rows_on_page < PAGE_SIZE;
            if last_page && page_read_short(page, rows_on_page, parsed.total) {
                self.tally.fail(format!(
                    "schools page {page} for {} returned {rows_on_page} of {PAGE_SIZE} rows while \
                     the response totals {}: the member walk stopped short",
                    state.code(),
                    parsed.total
                ));
            }
            for row in &parsed.rows {
                if self.at_limit() {
                    break;
                }
                self.process_school(state, org, row, &base_url).await?;
            }
            if last_page {
                ended = true;
                break;
            }
            page = page.saturating_add(1);
        }
        if !ended && !self.at_limit() {
            self.tally.fail(format!(
                "{} has more member pages than the {MAX_PAGES}-page walk reads",
                state.code()
            ));
        }
        Ok(())
    }

    fn at_limit(&self) -> bool {
        match self.options.limit {
            Some(limit) => self.tally.schools >= limit,
            None => false,
        }
    }

    pub(in crate::arbiter) async fn process_school(
        &mut self,
        state: UsJurisdiction,
        org: &str,
        row: &OrgSchool,
        base_url: &str,
    ) -> CrawlResult<()> {
        self.process_school_at(state, org, row, base_url, HOST)
            .await
    }

    async fn process_school_at(
        &mut self,
        state: UsJurisdiction,
        org: &str,
        row: &OrgSchool,
        base_url: &str,
        coach_host: &str,
    ) -> CrawlResult<()> {
        let Some((school, school_id)) =
            map_org_school(row, state, base_url, &self.options.observed_on)
        else {
            self.tally.fail(format!(
                "a member row of Arbiter organisation {org} for {} carries no school name \
                 (association id {:?})",
                state.code(),
                row.public_id
            ));
            return Ok(());
        };
        let key = completion_key(state, org, row.public_id);
        if key.as_ref().is_some_and(|key| self.done.contains(key)) {
            self.tally.skipped = self.tally.skipped.saturating_add(1);
            return Ok(());
        }
        let mut coaches = primary_coaches(row, &school_id, base_url, &self.options.observed_on);
        let mut recovery = match key.as_deref() {
            Some(key) => recovery::Recovery::load(self.ctx.store, &school_id, key)?,
            None => recovery::Recovery::default(),
        };
        let completion = match row.public_id.filter(|id| *id > 0) {
            Some(public_id) => {
                let acquisition = self
                    .collect_coaches(coach_host, org, public_id, &school, &mut recovery)
                    .await;
                coaches.extend(acquisition.coaches);
                acquisition.completion
            }
            None => Err(CrawlError::Schema {
                url: base_url.to_string(),
                detail: format!(
                    "{} has no association id to fetch coaches with",
                    school.name
                ),
            }),
        };
        self.finish_school(
            org,
            row,
            &school,
            key,
            coaches::CoachAcquisition {
                coaches,
                completion,
            },
            &recovery,
        )
    }

    async fn fetch_page(&mut self, url: &str) -> Option<FetchOutcome> {
        match self.ctx.fetcher.get(url, &self.fetch).await {
            Ok(outcome) => Some(outcome),
            Err(error) => {
                self.tally.fail(format!("fetch {url}: {error}"));
                None
            }
        }
    }

    fn persist_facts(
        &self,
        school: &CanonicalSchool,
        coaches: &[CanonicalCoach],
    ) -> CrawlResult<crate::recording::RowBatch<'_>> {
        let namespace = SourceNamespace::association_school(SOURCE_ID);
        let mut batch = self.ctx.write_batch();
        batch.append_many(Table::Schools, std::slice::from_ref(school))?;
        batch.append_many(
            Table::SourceObservations,
            self.ctx.school_observation(&namespace, school).as_slice(),
        )?;
        batch.append_many(Table::Coaches, coaches)?;
        Ok(batch)
    }

    fn write(
        &mut self,
        org: &str,
        row: &OrgSchool,
        school: &CanonicalSchool,
        coaches: &[CanonicalCoach],
        key: &str,
    ) -> CrawlResult<()> {
        let mut batch = self.persist_facts(school, coaches)?;
        batch.journal_done(
            JOURNAL,
            key,
            &json!({
                "org": org,
                "state": school.state.map(UsJurisdiction::code),
                "school": school.name,
                "association_id": row.public_id,
                "arbiter_org_id": row.org_id,
                "coach_rows": coaches.len(),
                "observed_on": self.options.observed_on,
            }),
        )?;
        batch.commit()?;
        self.done.insert(key.to_string());
        self.tally.coaches = self.tally.coaches.saturating_add(coaches.len());
        Ok(())
    }
}

fn page_read_short(page: u64, rows_on_page: u64, total: u64) -> bool {
    page.saturating_sub(1)
        .saturating_mul(PAGE_SIZE)
        .saturating_add(rows_on_page)
        < total
}

pub(in crate::arbiter) fn completion_key(
    state: UsJurisdiction,
    org: &str,
    public_id: Option<u64>,
) -> Option<String> {
    public_id
        .filter(|id| *id > 0)
        .map(|id| format!("{}:{org}:{id}", state.code()))
}

fn response_text<'a>(outcome: &'a FetchOutcome, subject: &str) -> CrawlResult<&'a str> {
    std::str::from_utf8(&outcome.body).map_err(|_| CrawlError::Schema {
        url: outcome.url.clone(),
        detail: format!("{subject} response is not UTF-8"),
    })
}

fn primary_coaches(
    row: &OrgSchool,
    school_id: &census_domain::model::SchoolId,
    base_url: &str,
    observed_on: &str,
) -> Vec<CanonicalCoach> {
    row.primary_contact
        .as_ref()
        .and_then(|contact| map_primary_contact(contact, school_id, base_url, observed_on))
        .into_iter()
        .collect()
}
