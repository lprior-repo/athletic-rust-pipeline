use super::nsaa::{nsaa_school_url, parse_nsaa_directory, NsaaRow, NsaaSchool};
use super::nsaa_coaches::{nsaa_coaches, parse_nsaa_row, parse_nsaa_school};
use super::parse::{email_regex, split_person_names};
use super::{NSAA_COACHES_PHASE, NSAA_SCHOOLS_PHASE};
use crate::net::FetchOptions;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{CanonicalCoach, CanonicalSchool, CoachRole, SourceNamespace};
use census_store::{StoreBatch, Table};

fn named_block<'a>(blocks: &'a [NsaaSchool], name: &str) -> Option<&'a NsaaSchool> {
    blocks
        .iter()
        .find(|entry| entry.name == name)
        .or_else(|| blocks.first())
}

fn coach_roles(coaches: &[CanonicalCoach]) -> (usize, usize) {
    let ad_rows = coaches
        .iter()
        .filter(|coach| coach.role == CoachRole::AthleticDirector)
        .count();
    let sport_rows = coaches
        .iter()
        .filter(|coach| coach.role == CoachRole::HeadCoach)
        .count();
    (ad_rows, sport_rows)
}

fn journal_school(
    batch: &mut StoreBatch<'_>,
    key: &str,
    published_rows: usize,
    coach_count: usize,
) -> CrawlResult<()> {
    batch.journal_done(
        NSAA_SCHOOLS_PHASE,
        key,
        &serde_json::json!({ "published_rows": published_rows }),
    )?;
    batch.journal_done(
        NSAA_COACHES_PHASE,
        key,
        &serde_json::json!({ "coach_rows": coach_count }),
    )?;
    Ok(())
}

#[derive(Default)]
pub(super) struct NsaaWalk {
    observed_on: String,
    processed: usize,
    resumed: usize,
    failed: usize,
    school_rows: usize,
    coach_rows: usize,
    ad_rows: usize,
    sport_rows: usize,
    slots: usize,
    slots_named: usize,
    rows_with_email: usize,
    coach_rows_with_email: usize,
    role_rows: usize,
}

impl NsaaWalk {
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
                report.note(format!("nsaa: {url} failed: {error}"));
                None
            }
        }
    }

    pub(super) async fn visit(
        &mut self,
        ctx: &AdapterContext<'_>,
        fetch: &FetchOptions,
        report: &mut AdapterReport,
        name: &str,
    ) -> CrawlResult<()> {
        let url = nsaa_school_url(name);
        let Some(html) = self.page(ctx, fetch, report, &url).await else {
            return Ok(());
        };
        let blocks = parse_nsaa_directory(&html)?;
        let Some(entry) = named_block(&blocks, name) else {
            self.failed = self.failed.saturating_add(1);
            report.errors = report.errors.saturating_add(1);
            report.note(format!("nsaa: {url} carried no school block"));
            return Ok(());
        };
        let (school, school_id) = parse_nsaa_school(entry, &url, &self.observed_on);
        let coaches = nsaa_coaches(entry, &school_id, &url, &self.observed_on)?;
        self.tally(entry)?;
        self.record(ctx, name, school, coaches, entry.roles.len())
    }

    fn tally(&mut self, entry: &NsaaSchool) -> CrawlResult<()> {
        for role in &entry.roles {
            self.role_rows = self.role_rows.saturating_add(1);
            if email_regex()?.is_match(&role.name) {
                self.rows_with_email = self.rows_with_email.saturating_add(1);
                if parse_nsaa_row(&role.label).is_some() {
                    self.coach_rows_with_email = self.coach_rows_with_email.saturating_add(1);
                }
            }
            if matches!(
                parse_nsaa_row(&role.label),
                Some(NsaaRow::SportCoach { .. })
            ) {
                self.slots = self.slots.saturating_add(1);
                if !split_person_names(&role.name)?.is_empty() {
                    self.slots_named = self.slots_named.saturating_add(1);
                }
            }
        }
        Ok(())
    }

    fn record(
        &mut self,
        ctx: &AdapterContext<'_>,
        name: &str,
        school: CanonicalSchool,
        coaches: Vec<CanonicalCoach>,
        published_rows: usize,
    ) -> CrawlResult<()> {
        let (ad_rows, sport_rows) = coach_roles(&coaches);
        self.ad_rows = self.ad_rows.saturating_add(ad_rows);
        self.sport_rows = self.sport_rows.saturating_add(sport_rows);
        let coach_count = coaches.len();

        let mut batch = ctx.store.write_batch();
        batch.append_many(Table::Schools, std::slice::from_ref(&school))?;
        batch.append_many(
            Table::SourceObservations,
            ctx.school_observation(
                &SourceNamespace::association_school(super::NSAA_ADAPTER_ID),
                &school,
            )
            .as_slice(),
        )?;
        batch.append_many(Table::Coaches, &coaches)?;
        self.school_rows = self.school_rows.saturating_add(1);
        self.coach_rows = self.coach_rows.saturating_add(coach_count);

        journal_school(
            &mut batch,
            &format!("NE:{name}"),
            published_rows,
            coach_count,
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
            detail: "nsaa school count exceeds u64".to_string(),
        })?;
        let coach_rows = u64::try_from(self.coach_rows).map_err(|_| CrawlError::Arithmetic {
            detail: "nsaa coach count exceeds u64".to_string(),
        })?;
        self.summary(report, members, coach_rows);
        Ok((school_rows, coach_rows))
    }

    fn summary(&self, report: &mut AdapterReport, members: usize, coach_rows: u64) {
        let NsaaWalk {
            processed,
            resumed,
            failed,
            ad_rows,
            sport_rows,
            slots,
            slots_named,
            rows_with_email,
            coach_rows_with_email,
            role_rows,
            ..
        } = self;
        report.note(format!(
            "nsaa: {processed} of {members} member schools parsed ({resumed} already journalled, \
             {failed} failed); {coach_rows} coach rows ({ad_rows} athletic/activities directors, \
             {sport_rows} sport rows); TF/XC coach slots named {slots_named}/{slots}"
        ));
        report.note(format!(
            "nsaa: {rows_with_email} of {role_rows} directory rows carry an email string \
             ({coach_rows_with_email} of them in coach/AD rows); \
             names only: provider publishes no coach email"
        ));
    }
}
