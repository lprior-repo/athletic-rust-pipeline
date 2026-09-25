//! The MSHSL run: its resume set, request inputs, tallies and per-school steps.
use crate::mshsl::map::{ad_coaches, ad_role, provider_key, school_domains, school_entities};
use crate::mshsl::parse::{
    listing_page_url, parse_next_listing_page, parse_school_detail, parse_school_list,
    school_page_url, SchoolDetail, SchoolListRow,
};
use crate::mshsl::{Options, MAX_LISTING_PAGES, SOURCE_ID};
use crate::net::{FetchOptions, FetchStats};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, SchoolId, SourceNamespace,
};
use census_domain::UsJurisdiction;
use census_store::{StoreBatch, Table};
use serde_json::json;
use std::collections::HashSet;

use super::{collect_team_coaches, count, fetch_options};

/// One run's inputs, resume set, tallies and the report being built.
pub(super) struct MshslRun<'a> {
    ctx: &'a AdapterContext<'a>,
    options: &'a Options,
    report: AdapterReport,
    wanted: HashSet<String>,
    done: HashSet<String>,
    fetch: FetchOptions,
    processed: usize,
    skipped: usize,
    unparsed: usize,
    ad_rows: usize,
    coach_rows: usize,
    with_email: u64,
    office_roles: usize,
}

impl<'a> MshslRun<'a> {
    /// Gather this run's inputs and resume set; `None` when the states exclude Minnesota.
    pub(super) fn start(
        ctx: &'a AdapterContext<'a>,
        options: &'a Options,
    ) -> CrawlResult<Option<Self>> {
        if !options.states.is_empty() && !options.states.contains(&UsJurisdiction::Minnesota) {
            return Ok(None);
        }
        let wanted = options
            .school_names
            .iter()
            .map(|name| normalize_name(name))
            .filter(|name| !name.is_empty())
            .collect();
        Ok(Some(Self {
            ctx,
            options,
            report: AdapterReport::new(SOURCE_ID, "schools"),
            wanted,
            done: ctx.store.journal_keys("mshsl_schools")?,
            fetch: fetch_options(ctx, options),
            processed: 0,
            skipped: 0,
            unparsed: 0,
            ad_rows: 0,
            coach_rows: 0,
            with_email: 0,
            office_roles: 0,
        }))
    }

    /// Walk the listing pages, one row at a time, until they run out or pagination stops.
    pub(super) async fn walk(&mut self) -> CrawlResult<()> {
        let mut page = 0usize;
        'pages: while page < MAX_LISTING_PAGES {
            let url = listing_page_url(page);
            // A fetch failure already names the URL; the page number rides in it.
            let outcome = self.ctx.fetcher.get(&url, &self.fetch).await?;
            let html = outcome.text();
            let rows = parse_school_list(&html);
            if rows.is_empty() {
                if page == 0 {
                    return Err(CrawlError::Schema {
                        url,
                        detail: "contained no school rows (markup change or empty page)"
                            .to_string(),
                    });
                }
                self.report.note(format!(
                    "listing page {url} contained no school rows; pagination stopped"
                ));
                break;
            }
            for row in rows {
                if self
                    .options
                    .limit
                    .is_some_and(|limit| self.processed >= limit)
                {
                    break 'pages;
                }
                self.process_row(&row).await?;
            }
            match parse_next_listing_page(&html, page) {
                Some(next) => page = next,
                None => break,
            }
        }
        Ok(())
    }

    /// Fetch and parse one listing row, then emit the school it names.
    async fn process_row(&mut self, row: &SchoolListRow) -> CrawlResult<()> {
        if !self.wanted.is_empty() && !self.wanted.contains(&normalize_name(&row.name)) {
            return Ok(());
        }
        let key = format!("MN:{}", row.slug);
        if self.done.contains(&key) {
            self.skipped = self.skipped.saturating_add(1);
            return Ok(());
        }
        let page_url = school_page_url(&row.slug);
        let fetched = self.ctx.fetcher.get(&page_url, &self.fetch).await;
        let detail = match fetched {
            Ok(outcome) => parse_school_detail(&outcome.text()),
            Err(error) => {
                self.report
                    .note(format!("school page {page_url}: {error:#}"));
                return Ok(());
            }
        };
        let Some((school, school_id)) =
            school_entities(row, &detail, &page_url, &self.options.observed_on)
        else {
            self.unparsed = self.unparsed.saturating_add(1);
            self.report.note(format!(
                "school page {page_url}: no school name in listing row or page"
            ));
            return Ok(());
        };
        self.emit_school(row, &school, &school_id, &detail).await
    }

    /// Emit one school's rows: the school, its AD rows, then its sport coaches and journals.
    async fn emit_school(
        &mut self,
        row: &SchoolListRow,
        school: &CanonicalSchool,
        school_id: &SchoolId,
        detail: &SchoolDetail,
    ) -> CrawlResult<()> {
        let page_url = school_page_url(&row.slug);
        let school_key = provider_key(row, detail);
        let observed_on = &self.options.observed_on;
        let ads = ad_coaches(detail, school_id, &school_key, &page_url, observed_on);
        let domains = school_domains(detail);
        let office = detail
            .admin
            .iter()
            .filter(|entry| ad_role(&entry.role).is_none());
        self.office_roles = self.office_roles.saturating_add(office.count());
        let (sport_coaches, notes) = match detail.school_id.as_deref() {
            Some(id) => {
                collect_team_coaches(self.ctx, &self.fetch, id, school_id, &domains, observed_on)
                    .await
            }
            None => (
                Vec::new(),
                vec![format!(
                    "school page {page_url}: no /group/<id>/ link, sport coaches skipped"
                )],
            ),
        };
        // One page for the unit: the school, both coach sets and the two journal entries commit
        // together, so the walk resumes on a school exactly when its rows are durable. The school
        // observation stays a direct write, the way every school arm writes it.
        let mut batch = self.ctx.store.write_batch();
        batch.append_many(Table::Schools, std::slice::from_ref(school))?;
        self.ctx
            .observe_school(&SourceNamespace::association_school(SOURCE_ID), school)?;
        batch.append_many(Table::Coaches, &ads)?;
        batch.append_many(Table::Coaches, &sport_coaches)?;
        for note in notes {
            self.report.note(format!("{}: {note}", school.name));
        }
        self.journal_school(&mut batch, row, detail, school, &ads, &sport_coaches)?;
        batch.commit()?;
        self.processed = self.processed.saturating_add(1);
        Ok(())
    }

    /// Journal one emitted school under both the school and the coach journals, in the page that holds
    /// its rows.
    fn journal_school(
        &mut self,
        batch: &mut StoreBatch<'_>,
        row: &SchoolListRow,
        detail: &SchoolDetail,
        school: &CanonicalSchool,
        ads: &[CanonicalCoach],
        sport_coaches: &[CanonicalCoach],
    ) -> CrawlResult<()> {
        let key = format!("MN:{}", row.slug);
        let school_key = provider_key(row, detail);
        let page_url = school_page_url(&row.slug);
        let with_email = ads
            .iter()
            .chain(sport_coaches.iter())
            .filter(|coach| coach.professional_email.is_some() || coach.personal_email.is_some())
            .count();
        self.ad_rows = self.ad_rows.saturating_add(ads.len());
        self.coach_rows = self.coach_rows.saturating_add(sport_coaches.len());
        self.with_email = self.with_email.saturating_add(count(with_email));
        batch.journal_done(
            "mshsl_schools",
            &key,
            &json!({
                "school": school.name,
                "school_id": school_key,
                "page_url": page_url,
                "city": row.city,
                "ad_rows": ads.len(),
                "sport_coach_rows": sport_coaches.len(),
                "with_email": with_email,
                "observed_on": self.options.observed_on,
            }),
        )?;
        batch.journal_done(
            "mshsl_coaches",
            &key,
            &json!({
                "school_id": school_key,
                "ad_rows": ads.len(),
                "sport_coach_rows": sport_coaches.len(),
                "with_email": with_email,
                "observed_on": self.options.observed_on,
            }),
        )?;
        Ok(())
    }

    /// Fill in the request deltas and the run notes, and hand back the report.
    pub(super) async fn finish(mut self, stats_before: FetchStats) -> AdapterReport {
        let stats_after = self.ctx.fetcher.stats().await;
        self.report.rows = count(self.processed);
        self.report.requests = stats_after.requests.saturating_sub(stats_before.requests);
        self.report.from_cache = stats_after
            .cache_hits
            .saturating_sub(stats_before.cache_hits);
        self.report.errors = stats_after
            .errors
            .saturating_sub(stats_before.errors)
            .saturating_add(count(self.unparsed));
        self.report.with_email = self.with_email;
        self.report.note(format!(
            "{} school(s) processed ({} already journalled): {} athletic-director row(s), {} sport-coach row(s), {} with a published email",
            self.processed, self.skipped, self.ad_rows, self.coach_rows, self.with_email
        ));
        self.report.note(format!(
            "{} Administration-block entry/entries were office roles (principal, superintendent, AD administrative assistant, trainer, advisors, Title IX, sports representatives) and were not emitted",
            self.office_roles
        ));
        self.report.note(
            "AD contacts come from the school page Administration block (Cloudflare-obfuscated addresses decoded locally); sport coaches come from /api/coaches/<team nid> reached through /jsonapi/views/teams/list_school, filtered to MSHSL coach levels; every well-formed published address is retained and classified, while phone columns remain withheld",
        );
        self.report
    }
}
