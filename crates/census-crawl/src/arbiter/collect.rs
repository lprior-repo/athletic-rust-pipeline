use super::map::{map_coach_row, map_org_school, map_primary_contact};
use super::parse::{
    credentials_in_bundle, parse_coach_rows, parse_org_schools, parse_token, OrgSchool,
};
use super::{org_for, BUNDLE_URL, HOST, MAX_PAGES, PAGE_SIZE, SOURCE_ID, TOKEN_SCOPE, TOKEN_URL};
use std::collections::HashSet;

use crate::net::{FetchOptions, FetchOutcome};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{CanonicalCoach, CanonicalSchool, SchoolId, SourceNamespace};
use census_domain::UsJurisdiction;
use census_store::Table;
use serde_json::json;

pub(super) const JOURNAL: &str = "arbiter_orgs";
const MAX_NOTES: usize = 32;

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub states: Vec<UsJurisdiction>,
    pub observed_on: String,
    pub limit: Option<usize>,
    pub refresh: bool,
}

#[derive(Default)]
pub(super) struct Tally {
    pub(super) schools: usize,
    pub(super) coaches: usize,
    pub(super) skipped: usize,
    pub(super) errors: usize,
    pub(super) notes: Vec<String>,
}

impl Tally {
    fn fail(&mut self, message: String) {
        self.errors = self.errors.saturating_add(1);
        if self.notes.len() < MAX_NOTES {
            self.notes.push(message);
        }
    }
}

pub(super) struct Run<'a> {
    pub(super) ctx: &'a AdapterContext<'a>,
    pub(super) options: &'a Options,
    pub(super) fetch: FetchOptions,
    pub(super) done: HashSet<String>,
    pub(super) tally: Tally,
}

pub(super) fn fetch_options(
    ctx: &AdapterContext<'_>,
    options: &Options,
    headers: Vec<(String, String)>,
) -> FetchOptions {
    let mut fetch = ctx.fetch_options();
    fetch.refresh = options.refresh || ctx.refresh;
    fetch.headers = headers;
    fetch
}

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let targets = targets(options)?;
    let org_count = targets.len();
    let stats_before = ctx.fetcher.stats().await;
    let token = mint_token(ctx, options).await?;
    let mut run = Run {
        ctx,
        options,
        fetch: fetch_options(
            ctx,
            options,
            vec![("Authorization".to_string(), format!("Bearer {token}"))],
        ),
        done: ctx.store.journal_keys(JOURNAL)?,
        tally: Tally::default(),
    };
    for (state, org) in targets {
        run.walk(state, org).await?;
    }
    let stats_after = ctx.fetcher.stats().await;
    let mut tally = run.tally;
    let mut report = AdapterReport::new(SOURCE_ID, "org_schools");
    report.rows = u64::try_from(tally.schools).unwrap_or(u64::MAX);
    report.requests = stats_after.requests.saturating_sub(stats_before.requests);
    report.from_cache = stats_after
        .cache_hits
        .saturating_sub(stats_before.cache_hits);
    report.errors = u64::try_from(tally.errors).unwrap_or(u64::MAX);
    report.notes = std::mem::take(&mut tally.notes);
    if tally.skipped > 0 {
        report.note(format!(
            "{} school(s) already journaled, skipped",
            tally.skipped
        ));
    }
    report.note(format!(
        "{} school(s) and {} coach row(s) over {org_count} Arbiter organisation(s)",
        tally.schools, tally.coaches
    ));
    Ok(report)
}

fn page_read_short(page: u64, rows_on_page: u64, total: u64) -> bool {
    page.saturating_sub(1)
        .saturating_mul(PAGE_SIZE)
        .saturating_add(rows_on_page)
        < total
}

pub(super) fn targets(options: &Options) -> CrawlResult<Vec<(UsJurisdiction, &'static str)>> {
    let requested: Vec<UsJurisdiction> = if options.states.is_empty() {
        super::covered_states().collect()
    } else {
        options.states.clone()
    };
    let mut targets = Vec::with_capacity(requested.len());
    for state in requested {
        let org = org_for(state).ok_or_else(|| CrawlError::Invariant {
            detail: format!("no Arbiter organisation is registered for {}", state.code()),
        })?;
        if targets.iter().any(|(seen, _)| *seen == state) {
            continue;
        }
        targets.push((state, org));
    }
    Ok(targets)
}

async fn mint_token(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<String> {
    let bundle = ctx
        .fetcher
        .get(BUNDLE_URL, &fetch_options(ctx, options, Vec::new()))
        .await
        .map_err(|error| CrawlError::Invariant {
            detail: format!(
                "the Arbiter directory bundle {BUNDLE_URL} could not be read: {error}; the asset \
                 name is pinned and changes when Arbiter redeploys"
            ),
        })?;
    let Some((client_id, client_secret)) = credentials_in_bundle(&bundle.text()) else {
        return Err(CrawlError::Schema {
            url: BUNDLE_URL.to_string(),
            detail: "no client_id/client_secret pair in the published bundle".to_string(),
        });
    };
    let form = vec![
        ("client_id".to_string(), client_id),
        ("client_secret".to_string(), client_secret),
        ("grant_type".to_string(), "client_credentials".to_string()),
        ("scope".to_string(), TOKEN_SCOPE.to_string()),
    ];
    let mut token_fetch = ctx.fetch_options();
    token_fetch.refresh = true;
    let body = ctx
        .fetcher
        .post_form(TOKEN_URL, &form, &token_fetch)
        .await?;
    parse_token(&body.text(), TOKEN_URL)
}

impl Run<'_> {
    pub(super) async fn walk(&mut self, state: UsJurisdiction, org: &str) -> CrawlResult<()> {
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

    pub(super) async fn process_school(
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
