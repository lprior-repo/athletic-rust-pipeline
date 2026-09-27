
use super::nd::{
    parse_nd_offerings, parse_nd_school_page, parse_nd_staff, NdOffering, NdSchoolRef, NdStaffRole,
};
use super::nd_coaches::{nd_ad_coaches, nd_sport_coaches, parse_nd_sport};
use super::parse::email_regex;
use super::{ND_COACHES_PHASE, ND_SCHOOLS_PHASE};
use crate::net::FetchOptions;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{CanonicalSchool, SchoolId, SourceNamespace};
use census_store::{StoreBatch, Table};

struct NdPage {
    school: CanonicalSchool,
    school_id: SchoolId,
    staff: Vec<NdStaffRole>,
    offerings: Vec<NdOffering>,
}

fn journal_school(
    batch: &mut StoreBatch<'_>,
    key: &str,
    slug: &str,
    offerings: usize,
    coach_rows: usize,
    ad_rows: usize,
) -> CrawlResult<()> {
    batch.journal_done(
        ND_SCHOOLS_PHASE,
        key,
        &serde_json::json!({ "slug": slug, "offerings": offerings }),
    )?;
    batch.journal_done(
        ND_COACHES_PHASE,
        key,
        &serde_json::json!({ "coach_rows": coach_rows, "ad_rows": ad_rows }),
    )?;
    Ok(())
}

#[derive(Default)]
pub(super) struct NdWalk {
    observed_on: String,
    processed: usize,
    resumed: usize,
    failed: usize,
    school_rows: usize,
    coach_rows: usize,
    ad_rows: usize,
    slots: usize,
    slots_named: usize,
    pages_with_email: usize,
}

impl NdWalk {
    pub(super) fn new(observed_on: String) -> Self {
        Self {
            observed_on,
            ..Self::default()
        }
    }

    pub(super) fn limit_reached(&self, limit: Option<usize>) -> bool {
        limit.is_some_and(|limit| self.processed >= limit)
    }

    pub(super) fn note_resumed(&mut self) {
        self.resumed = self.resumed.saturating_add(1);
    }

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

    pub(super) async fn visit(
        &mut self,
        ctx: &AdapterContext<'_>,
        fetch: &FetchOptions,
        report: &mut AdapterReport,
        member: &NdSchoolRef,
    ) -> CrawlResult<()> {
        let url = member.url();
        let Some(html) = self.page(ctx, fetch, report, &url).await else {
            return Ok(());
        };
        let Some(page) = self.parse_page(&html, member, &url, report)? else {
            return Ok(());
        };
        self.record(ctx, &url, member, page)
    }

    fn parse_page(
        &mut self,
        html: &str,
        member: &NdSchoolRef,
        url: &str,
        report: &mut AdapterReport,
    ) -> CrawlResult<Option<NdPage>> {
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

    fn record(
        &mut self,
        ctx: &AdapterContext<'_>,
        url: &str,
        member: &NdSchoolRef,
        page: NdPage,
    ) -> CrawlResult<()> {
        let mut coaches = nd_ad_coaches(&page.staff, &page.school_id, url, &self.observed_on);
        let ad_count = coaches.len();
        coaches.extend(nd_sport_coaches(
            &page.offerings,
            &page.school_id,
            url,
            &self.observed_on,
        ));
        self.tally_offerings(&page.offerings);
        self.ad_rows = self.ad_rows.saturating_add(ad_count);

        let mut batch = ctx.store.write_batch();
        batch.append_many(Table::Schools, std::slice::from_ref(&page.school))?;
        batch.append_many(Table::SourceObservations, ctx.school_observation(&SourceNamespace::association_school(super::ND_ADAPTER_ID),
        &page.school,).as_slice())?;
        batch.append_many(Table::Coaches, &coaches)?;
        self.school_rows = self.school_rows.saturating_add(1);
        self.coach_rows = self.coach_rows.saturating_add(coaches.len());

        let key = format!("ND:{}", member.id);
        journal_school(
            &mut batch,
            &key,
            &member.slug,
            page.offerings.len(),
            coaches.len(),
            ad_count,
        )?;
        batch.commit()?;
        self.processed = self.processed.saturating_add(1);
        Ok(())
    }

    pub(super) fn publish(
        &self,
        report: &mut AdapterReport,
        members: usize,
    ) -> CrawlResult<(u64, u64)> {
        let school_rows = u64::try_from(self.school_rows).map_err(|_| CrawlError::Arithmetic {
            detail: "ndhsaa school count exceeds u64".to_string(),
        })?;
        let coach_rows = u64::try_from(self.coach_rows).map_err(|_| CrawlError::Arithmetic {
            detail: "ndhsaa coach count exceeds u64".to_string(),
        })?;
        self.summary(report, members, coach_rows);
        Ok((school_rows, coach_rows))
    }

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
