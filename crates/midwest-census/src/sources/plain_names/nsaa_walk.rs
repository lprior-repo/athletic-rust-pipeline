//! The Nebraska half's per-school walk: fetch one member page, tally its published rows, journal
//! the school, and keep the entity rows the run writes once the directory form is exhausted.
//!
//! Split out of `nsaa_coaches` when that file's walk outgrew the repository's file budget; the
//! collection entry point and the row/school parsers stay there.

use super::nsaa::{nsaa_school_url, parse_nsaa_directory, NsaaRow, NsaaSchool};
use super::nsaa_coaches::{nsaa_coaches, parse_nsaa_row, parse_nsaa_school};
use super::parse::{email_regex, split_person_names};
use super::{NSAA_COACHES_PHASE, NSAA_SCHOOLS_PHASE};
use census_domain::model::{CanonicalCoach, CanonicalSchool, CoachRole};
use crate::net::FetchOptions;
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::{Context, Result};

/// The school block `name` names, or the page's only block.
fn named_block<'a>(blocks: &'a [NsaaSchool], name: &str) -> Option<&'a NsaaSchool> {
    blocks
        .iter()
        .find(|entry| entry.name == name)
        .or_else(|| blocks.first())
}

/// The athletic-director and head-coach row counts of one school's coach rows.
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

/// Journal both NSAA phases for one walked school.
fn journal_school(
    ctx: &AdapterContext<'_>,
    key: &str,
    published_rows: usize,
    coach_count: usize,
) -> Result<()> {
    ctx.store.journal_done(
        NSAA_SCHOOLS_PHASE,
        key,
        &serde_json::json!({ "published_rows": published_rows }),
    )?;
    ctx.store.journal_done(
        NSAA_COACHES_PHASE,
        key,
        &serde_json::json!({ "coach_rows": coach_count }),
    )
}

/// One walk of the NSAA member directory: the entity rows it accumulates and its report counters.
#[derive(Default)]
pub(super) struct NsaaWalk {
    schools: Vec<CanonicalSchool>,
    coaches: Vec<CanonicalCoach>,
    observed_on: String,
    processed: usize,
    resumed: usize,
    failed: usize,
    ad_rows: usize,
    sport_rows: usize,
    slots: usize,
    slots_named: usize,
    rows_with_email: usize,
    coach_rows_with_email: usize,
    role_rows: usize,
}

impl NsaaWalk {
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
                report.note(format!("nsaa: {url} failed: {error}"));
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
        name: &str,
    ) -> Result<()> {
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

    /// Count one page's published rows for the report lines.
    fn tally(&mut self, entry: &NsaaSchool) -> Result<()> {
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

    /// Append one school's entity rows, count its coach roles and journal both phases.
    fn record(
        &mut self,
        ctx: &AdapterContext<'_>,
        name: &str,
        school: CanonicalSchool,
        coaches: Vec<CanonicalCoach>,
        published_rows: usize,
    ) -> Result<()> {
        let (ad_rows, sport_rows) = coach_roles(&coaches);
        self.ad_rows = self.ad_rows.saturating_add(ad_rows);
        self.sport_rows = self.sport_rows.saturating_add(sport_rows);
        let coach_count = coaches.len();
        self.coaches.extend(coaches);
        self.schools.push(school);

        journal_school(ctx, &format!("NE:{name}"), published_rows, coach_count)?;
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
            u64::try_from(self.schools.len()).context("nsaa school count exceeds u64")?;
        let coach_rows =
            u64::try_from(self.coaches.len()).context("nsaa coach count exceeds u64")?;
        ctx.store
            .append_many(Table::Schools, &self.schools)
            .context("writing nsaa schools")?;
        ctx.store
            .append_many(Table::Coaches, &self.coaches)
            .context("writing nsaa coaches")?;

        self.summary(report, members, coach_rows);
        Ok((school_rows, coach_rows))
    }

    /// The two NSAA summary lines: parse coverage and the email/name split.
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
