use super::super::map::{map_coach_row, map_org_school, map_primary_contact};
use super::super::parse::{parse_coach_rows, parse_org_schools, OrgSchool};
use super::super::{HOST, MAX_PAGES, PAGE_SIZE, SOURCE_ID};
use crate::net::{FetchOptions, FetchOutcome};
use crate::{AdapterContext, CrawlResult};
use census_domain::model::{CanonicalCoach, CanonicalSchool, SchoolId, SourceNamespace};
use census_domain::UsJurisdiction;
use census_store::Table;
use serde_json::json;
use std::collections::HashSet;

pub(in crate::arbiter) const JOURNAL: &str = "arbiter_orgs";
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
            let parsed = match parse_org_schools(&outcome.text(), &url) {
                Ok(parsed) => parsed,
                Err(error) => {
                    self.tally
                        .fail(format!("schools page {page} for {}: {error}", state.code()));
                    ended = true;
                    break;
                }
            };
            let rows_on_page = u64::try_from(parsed.rows.len()).unwrap_or(u64::MAX);
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
        if self.done.contains(school_id.as_str()) {
            self.tally.skipped = self.tally.skipped.saturating_add(1);
            return Ok(());
        }
        let mut coaches: Vec<CanonicalCoach> = Vec::new();
        if let Some(contact) = &row.primary_contact {
            if let Some(contact) =
                map_primary_contact(contact, &school_id, base_url, &self.options.observed_on)
            {
                coaches.push(contact);
            }
        }
        if let Some(public_id) = row.public_id {
            coaches.extend(
                self.collect_coaches(org, public_id, &school.name, &school_id)
                    .await,
            );
        }
        self.write(org, row, &school, &coaches)?;
        self.tally.schools = self.tally.schools.saturating_add(1);
        Ok(())
    }

    async fn collect_coaches(
        &mut self,
        org: &str,
        public_id: u64,
        school_name: &str,
        school_id: &SchoolId,
    ) -> Vec<CanonicalCoach> {
        let mut coaches: Vec<CanonicalCoach> = Vec::new();
        let mut page = 1u64;
        let mut ended = false;
        while page <= MAX_PAGES {
            let url = format!(
                "{HOST}/api/v2/legacy/public/{org}/coaches?filter.EntityId={public_id}&&pageSize={PAGE_SIZE}&pageNumber={page}"
            );
            let Some(outcome) = self.fetch_page(&url).await else {
                ended = true;
                break;
            };
            let parsed = match parse_coach_rows(&outcome.text(), &url) {
                Ok(parsed) => parsed,
                Err(error) => {
                    self.tally
                        .fail(format!("coaches for {school_name}: {error}"));
                    ended = true;
                    break;
                }
            };
            let rows_on_page = u64::try_from(parsed.rows.len()).unwrap_or(u64::MAX);
            for row in &parsed.rows {
                if let Some(coach) = map_coach_row(row, school_id, &url, &self.options.observed_on)
                {
                    coaches.push(coach);
                }
            }
            if rows_on_page < PAGE_SIZE {
                if page_read_short(page, rows_on_page, parsed.total) {
                    self.tally.fail(format!(
                        "coaches for {school_name}: page {page} returned {rows_on_page} of \
                         {PAGE_SIZE} rows while the response totals {}",
                        parsed.total
                    ));
                }
                ended = true;
                break;
            }
            page = page.saturating_add(1);
        }
        if !ended {
            self.tally.fail(format!(
                "coaches for {school_name} have more pages than the {MAX_PAGES}-page walk reads"
            ));
        }
        coaches
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

    fn write(
        &mut self,
        org: &str,
        row: &OrgSchool,
        school: &CanonicalSchool,
        coaches: &[CanonicalCoach],
    ) -> CrawlResult<()> {
        let namespace = SourceNamespace::association_school(SOURCE_ID);
        let mut batch = self.ctx.write_batch();
        batch.append_many(Table::Schools, std::slice::from_ref(school))?;
        batch.append_many(
            Table::SourceObservations,
            self.ctx.school_observation(&namespace, school).as_slice(),
        )?;
        batch.append_many(Table::Coaches, coaches)?;
        batch.journal_done(
            JOURNAL,
            school.id.as_str(),
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
