//! The North Dakota half's per-school walk: fetch one member page, read its heading, staff and
//! offering rows, journal the school, and keep the entity rows the run writes once the index is
//! exhausted.
//!
//! Split out of `nd_coaches` when that file's walk outgrew the repository's file budget; the
//! collection entry point and the row parsers stay there.

use super::nd::{
    parse_nd_offerings, parse_nd_school_page, parse_nd_staff, NdOffering, NdSchoolRef, NdStaffRole,
};
use super::nd_coaches::{nd_ad_coaches, nd_sport_coaches, parse_nd_sport};
use super::parse::email_regex;
use super::{ND_COACHES_PHASE, ND_SCHOOLS_PHASE};
use crate::net::FetchOptions;
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::{Context, Result};
use census_domain::model::{CanonicalCoach, CanonicalSchool, SchoolId};

/// One parsed NDHSAA school page: its heading and the row sets the entity builders read.
struct NdPage {
    school: CanonicalSchool,
    school_id: SchoolId,
    staff: Vec<NdStaffRole>,
    offerings: Vec<NdOffering>,
}

/// Journal both NDHSAA phases for one walked school.
fn journal_school(
    ctx: &AdapterContext<'_>,
    key: &str,
    slug: &str,
    offerings: usize,
    coach_rows: usize,
    ad_rows: usize,
) -> Result<()> {
    ctx.store.journal_done(
        ND_SCHOOLS_PHASE,
        key,
        &serde_json::json!({ "slug": slug, "offerings": offerings }),
    )?;
    ctx.store.journal_done(
        ND_COACHES_PHASE,
        key,
        &serde_json::json!({ "coach_rows": coach_rows, "ad_rows": ad_rows }),
    )
}

/// One walk of the NDHSAA member index: the entity rows it accumulates and its report counters.
#[derive(Default)]
pub(super) struct NdWalk {
    schools: Vec<CanonicalSchool>,
    coaches: Vec<CanonicalCoach>,
    observed_on: String,
    processed: usize,
    resumed: usize,
    failed: usize,
    ad_rows: usize,
    slots: usize,
    slots_named: usize,
    pages_with_email: usize,
}

impl NdWalk {
    /// A walk whose entities cite `observed_on`.
    pub(super) fn new(observed_on: String) -> Self {
        Self {
            observed_on,
            ..Self::default()
        }
    }

    /// Whether the operator's row limit has already been reached.
    pub(super) fn limit_reached(&self, limit: Option<usize>) -> bool {
        limit.is_some_and(|limit| self.processed >= limit)
    }

    /// Count one member school as already journalled.
    pub(super) fn note_resumed(&mut self) {
        self.resumed = self.resumed.saturating_add(1);
    }

    /// Fetch one member page's HTML; `None` means the failure is already on the report.
    async fn page(
        &mut self,
        ctx: &AdapterContext<'_>,
        fetch: &FetchOptions,
        report: &mut AdapterReport,
        url: &str,
    ) -> Option<String> {
        match ctx.fetcher.get(url, fetch).await {
            Ok(outcome) => Some(outcome.text()),
            Err(error) => {
                self.failed = self.failed.saturating_add(1);
                report.errors = report.errors.saturating_add(1);
                report.note(format!("ndhsaa: {url} failed: {error}"));
                None
            }
        }
    }

    /// Fetch, parse and record one member school: its entity rows, tallies and journal entries.
    pub(super) async fn visit(
        &mut self,
        ctx: &AdapterContext<'_>,
        fetch: &FetchOptions,
        report: &mut AdapterReport,
        member: &NdSchoolRef,
    ) -> Result<()> {
        let url = member.url();
        let Some(html) = self.page(ctx, fetch, report, &url).await else {
            return Ok(());
        };
        let Some(page) = self.parse_page(&html, member, &url, report)? else {
            return Ok(());
        };
        self.record(ctx, &url, member, page)
    }

    /// Parse one member page into its heading and row sets; `None` means it carried no heading.
    fn parse_page(
        &mut self,
        html: &str,
        member: &NdSchoolRef,
        url: &str,
        report: &mut AdapterReport,
    ) -> Result<Option<NdPage>> {
        let Some((school, school_id)) = parse_nd_school_page(html, member, &self.observed_on)?
        else {
            self.failed = self.failed.saturating_add(1);
            report.errors = report.errors.saturating_add(1);
            report.note(format!("ndhsaa: {url} carried no school heading"));
            return Ok(None);
        };
        if email_regex()?.is_match(html) {
            self.pages_with_email = self.pages_with_email.saturating_add(1);
        }
        Ok(Some(NdPage {
            school,
            school_id,
            staff: parse_nd_staff(html)?,
            offerings: parse_nd_offerings(html)?,
        }))
    }

    /// Count one page's published TF/XC offering slots, named or blank.
    fn tally_offerings(&mut self, offerings: &[NdOffering]) {
        for offering in offerings {
            if parse_nd_sport(&offering.label).is_some() {
                self.slots = self.slots.saturating_add(1);
                if !offering.coaches.is_empty() {
                    self.slots_named = self.slots_named.saturating_add(1);
                }
            }
        }
    }

    /// Append one page's entity rows, count its coach roles and journal both phases.
    fn record(
        &mut self,
        ctx: &AdapterContext<'_>,
        url: &str,
        member: &NdSchoolRef,
        page: NdPage,
    ) -> Result<()> {
        let ads = nd_ad_coaches(&page.staff, &page.school_id, url, &self.observed_on);
        let sport_coaches =
            nd_sport_coaches(&page.offerings, &page.school_id, url, &self.observed_on);
        self.tally_offerings(&page.offerings);
        let ad_count = ads.len();
        let sport_count = sport_coaches.len();
        self.ad_rows = self.ad_rows.saturating_add(ad_count);
        self.coaches.extend(ads);
        self.coaches.extend(sport_coaches);
        self.schools.push(page.school);

        let key = format!("ND:{}", member.id);
        let coach_rows = sport_count.saturating_add(ad_count);
        journal_school(
            ctx,
            &key,
            &member.slug,
            page.offerings.len(),
            coach_rows,
            ad_count,
        )?;
        self.processed = self.processed.saturating_add(1);
        Ok(())
    }

    /// Write the walked entities and emit the two summary lines.
    pub(super) fn publish(
        &self,
        ctx: &AdapterContext<'_>,
        report: &mut AdapterReport,
        members: usize,
    ) -> Result<(u64, u64)> {
        let school_rows =
            u64::try_from(self.schools.len()).context("ndhsaa school count exceeds u64")?;
        let coach_rows =
            u64::try_from(self.coaches.len()).context("ndhsaa coach count exceeds u64")?;
        ctx.store
            .append_many(Table::Schools, &self.schools)
            .context("writing ndhsaa schools")?;
        ctx.store
            .append_many(Table::Coaches, &self.coaches)
            .context("writing ndhsaa coaches")?;

        self.summary(report, members, coach_rows);
        Ok((school_rows, coach_rows))
    }

    /// The two NDHSAA summary lines: parse coverage and the page email split.
    fn summary(&self, report: &mut AdapterReport, members: usize, coach_rows: u64) {
        let NdWalk {
            processed,
            resumed,
            failed,
            ad_rows,
            slots,
            slots_named,
            pages_with_email,
            ..
        } = self;
        report.note(format!(
            "ndhsaa: {processed} of {members} member schools parsed ({resumed} already journalled, \
             {failed} failed); {coach_rows} coach rows ({ad_rows} athletic/activities directors); \
             TF/XC coach slots named {slots_named}/{slots}; {members} listings"
        ));
        report.note(format!(
            "ndhsaa: {pages_with_email} of {processed} school pages walked contain any email string; \
             names only: provider publishes no coach email (every coach entity has professional_email=None)"
        ));
    }
}
