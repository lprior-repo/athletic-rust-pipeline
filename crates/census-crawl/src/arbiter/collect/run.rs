use super::super::map::{map_org_school, map_primary_contact};
use super::super::parse::{parse_org_schools, OrgSchool};
use super::super::{HOST, MAX_PAGES, PAGE_SIZE};
use crate::directory::acquisition::{bounded, detail, text};
use crate::net::{FetchOptions, FetchOutcome};
use crate::{AdapterContext, CrawlError, CrawlResult};
use census_domain::model::{CanonicalCoach, CanonicalSchool};
use census_domain::UsJurisdiction;
use census_store::Table;
use futures::{stream, StreamExt, TryStreamExt};

mod coaches;
mod persistence;
mod recovery;
#[cfg(test)]
mod tests;

pub(in crate::arbiter) const JOURNAL: &str = "arbiter_owned_coaches_v2";

#[derive(Default)]
pub(in crate::arbiter) struct Tally {
    pub(in crate::arbiter) schools: usize,
    pub(in crate::arbiter) coaches: usize,
    pub(in crate::arbiter) skipped: usize,
    pub(in crate::arbiter) errors: usize,
    pub(in crate::arbiter) notes: Vec<String>,
    pub(in crate::arbiter) unfinished: Vec<String>,
    attempted: usize,
}

impl Tally {
    fn owe(&mut self, locator: &str) -> CrawlResult<()> {
        if !self.unfinished.iter().any(|entry| entry == locator) {
            bounded(&mut self.unfinished, locator.to_owned())?;
        }
        Ok(())
    }

    fn fail(&mut self, locator: &str, error: impl std::fmt::Display) -> CrawlResult<()> {
        self.errors = self.errors.saturating_add(1);
        self.owe(locator)?;
        if self.notes.len() < 5 {
            bounded(&mut self.notes, detail(locator, error)?)?;
        }
        Ok(())
    }
}

pub(in crate::arbiter) struct Run<'a> {
    pub(in crate::arbiter) ctx: &'a AdapterContext<'a>,
    pub(in crate::arbiter) options: &'a super::Options,
    pub(in crate::arbiter) fetch: FetchOptions,
    pub(in crate::arbiter) tally: Tally,
}

impl Run<'_> {
    pub(in crate::arbiter) async fn walk(
        &mut self,
        state: UsJurisdiction,
        org: &str,
    ) -> CrawlResult<()> {
        let base = format!("{HOST}/api/v2/organization/public/{org}/children");
        let base = base.as_str();
        let mut page = Some(1);
        stream::iter(1..=MAX_PAGES)
            .map(Ok::<_, CrawlError>)
            .try_fold((&mut *self, &mut page), |(run, next), current| async move {
                if next.is_some() {
                    *next = run.member_page(state, org, (base, current)).await?;
                }
                Ok((run, next))
            })
            .await?;
        if let Some(page) = page {
            self.tally.owe(&page_url(base, page))?;
        }
        Ok(())
    }

    async fn member_page(
        &mut self,
        state: UsJurisdiction,
        org: &str,
        page: (&str, u64),
    ) -> CrawlResult<Option<u64>> {
        let url = page_url(page.0, page.1);
        let Some(capture) = self.fetch_page(&url).await? else {
            return Ok(None);
        };
        let parsed = match text(&capture).and_then(|body| parse_org_schools(body, &url)) {
            Ok(parsed) => parsed,
            Err(error) => {
                self.tally.fail(&url, error)?;
                return Ok(None);
            }
        };
        let count = u64::try_from(parsed.rows.len()).map_err(|_| CrawlError::Arithmetic {
            detail: "member page size".into(),
        })?;
        let origin = (url.as_str(), HOST);
        let stamp = capture.fetched_at.as_str();
        stream::iter(&parsed.rows)
            .map(Ok::<_, CrawlError>)
            .try_fold(&mut *self, |run, row| async move {
                run.project((state, org, row), origin, stamp).await?;
                Ok(run)
            })
            .await?;
        if count < PAGE_SIZE {
            if page_read_short(page.1, count, parsed.total) {
                self.tally
                    .fail(&url, "member walk stopped short of the published total")?;
            }
            return Ok(None);
        }
        Ok(page.1.checked_add(1))
    }

    #[cfg(test)]
    pub(in crate::arbiter) async fn process_school(
        &mut self,
        state: UsJurisdiction,
        org: &str,
        row: &OrgSchool,
        base: &str,
    ) -> CrawlResult<()> {
        let ctx = self.ctx;
        self.project((state, org, row), (base, HOST), &ctx.observed_on)
            .await
    }

    #[cfg(test)]
    async fn process_school_at(
        &mut self,
        state: UsJurisdiction,
        org: &str,
        row: &OrgSchool,
        origin: (&str, &str),
    ) -> CrawlResult<()> {
        let ctx = self.ctx;
        self.project((state, org, row), origin, &ctx.observed_on)
            .await
    }

    async fn project(
        &mut self,
        subject: (UsJurisdiction, &str, &OrgSchool),
        origin: (&str, &str),
        stamp: &str,
    ) -> CrawlResult<()> {
        let (state, org, row) = subject;
        let locator = format!("{}#school={:?}", origin.0, row.public_id);
        if self
            .options
            .limit
            .is_some_and(|limit| self.tally.attempted >= limit)
        {
            return self.tally.owe(&locator);
        }
        self.tally.attempted = self.tally.attempted.saturating_add(1);
        let Some((school, id)) = map_org_school(row, state, origin.0, stamp) else {
            return self.tally.fail(&locator, "missing member school name");
        };
        let errors = self.tally.errors;
        let written = self.persist_owner(&school, &locator, stamp)?;
        let primary = primary_coaches(row, &id, origin.0, stamp);
        self.tally.coaches = self.tally.coaches.saturating_add(self.persist_rows(
            &locator,
            Table::Coaches,
            &primary,
        )?);
        let Some(key) = completion_key(state, org, row.public_id) else {
            self.tally.schools = self.tally.schools.saturating_add(written);
            return self.tally.fail(&locator, "missing public coach owner id");
        };
        let mut recovery = recovery::Recovery::load(self.ctx.store, &id, &key)?;
        let mut acquisition = self
            .coaches((origin.1, org), row.public_id, &school, &mut recovery)
            .await?;
        if written > 0 || acquisition.admitted > 0 {
            self.tally.schools = self.tally.schools.saturating_add(1);
        } else {
            self.tally.skipped = self.tally.skipped.saturating_add(1);
        }
        self.tally.coaches = self.tally.coaches.saturating_add(acquisition.admitted);
        acquisition.published = acquisition.published.saturating_add(primary.len());
        if errors != self.tally.errors && acquisition.completion.is_ok() {
            return Ok(());
        }
        self.finish(&key, &school, &recovery, acquisition)
    }

    fn finish(
        &self,
        key: &str,
        school: &CanonicalSchool,
        recovery: &recovery::Recovery,
        acquisition: coaches::Acquisition,
    ) -> CrawlResult<()> {
        match acquisition.completion {
            Ok(()) => self.complete(key, school, acquisition.published),
            Err(error) => {
                let mut batch = self.ctx.write_batch();
                recovery.persist(&mut batch, &school.id, key, &error)?;
                batch.commit()
            }
        }
    }

    async fn fetch_page(&mut self, url: &str) -> CrawlResult<Option<FetchOutcome>> {
        match self.ctx.fetcher.get(url, &self.fetch).await {
            Ok(capture) if capture.status == 200 => Ok(Some(capture)),
            Ok(capture) => {
                self.tally.fail(url, format!("HTTP {}", capture.status))?;
                Ok(None)
            }
            Err(error) => {
                self.tally.fail(url, error)?;
                Ok(None)
            }
        }
    }
}

fn page_url(base: &str, page: u64) -> String {
    format!("{base}?&pageSize={PAGE_SIZE}&pageNumber={page}")
}

fn page_read_short(page: u64, rows: u64, total: u64) -> bool {
    page.checked_sub(1)
        .and_then(|page| page.checked_mul(PAGE_SIZE))
        .and_then(|offset| offset.checked_add(rows))
        .is_none_or(|read| read < total)
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

fn primary_coaches(
    row: &OrgSchool,
    id: &census_domain::model::SchoolId,
    url: &str,
    stamp: &str,
) -> Vec<CanonicalCoach> {
    row.primary_contact
        .as_ref()
        .and_then(|contact| map_primary_contact(contact, id, url, stamp))
        .into_iter()
        .collect()
}
