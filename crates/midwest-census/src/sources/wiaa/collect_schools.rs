//! The school-page stage of the WIAA directory walk: which pages are still worth fetching, the
//! tally they feed, and the school plus coach rows they write.

use super::super::map::{school_entities, SchoolExtract};
use super::super::parse::{parse_school_page, IndexEntry, SchoolPage};
use super::super::{count, fetch_options, Options};
use crate::net::{FetchError, FetchOptions, FetchOutcome};
use crate::sources::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::normalize_name;
use census_store::Table;
use futures::stream::{self, StreamExt};
use std::collections::HashSet;

/// Per-run counters and transcripts for the directory walk.
#[derive(Debug, Default)]
pub(super) struct SchoolTally {
    pub(super) processed: usize,
    pub(super) skipped_done: usize,
    pub(super) skipped_filter: usize,
    pub(super) not_found: usize,
    pub(super) page_failures: usize,
    pub(super) coach_rows: usize,
    pub(super) with_email: u64,
    pub(super) skipped_admin_roles: Vec<String>,
    pub(super) skipped_coach_rows: usize,
}

/// Fetch the school pages still worth reading: the requested names, minus the journaled ones.
pub(super) async fn plan_schools(
    ctx: &AdapterContext<'_>,
    options: &Options,
    index: &[IndexEntry],
    done: &HashSet<String>,
    tally: &mut SchoolTally,
) -> Vec<(usize, Result<FetchOutcome, FetchError>)> {
    let wanted: Option<HashSet<String>> = if options.school_names.is_empty() {
        None
    } else {
        Some(
            options
                .school_names
                .iter()
                .map(|name| normalize_name(name))
                .collect(),
        )
    };

    // Phase 1: identify eligible schools (skip already journaled, apply filter).
    let eligible: Vec<(usize, &IndexEntry)> = index
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            if let Some(wanted) = wanted.as_ref() {
                if !wanted.contains(&normalize_name(&entry.name)) {
                    tally.skipped_filter = tally.skipped_filter.saturating_add(1);
                    return false;
                }
            }
            let key = format!("WI:{}", entry.org_id);
            if done.contains(&key) {
                tally.skipped_done = tally.skipped_done.saturating_add(1);
                return false;
            }
            true
        })
        .collect();

    // Phase 2: fetch all school pages in parallel (bounded concurrency, shared bound).
    // Each fetch is independent — same host, but rate-limited by the fetcher's gate.
    const SCHOOL_CONCURRENCY: usize = crate::sources::CONCURRENCY_BOUND;
    stream::iter(eligible)
        .map(|(idx, entry)| {
            let url = entry.page_url();
            let page_options = FetchOptions {
                allow_not_found: true,
                ..fetch_options(ctx, options)
            };
            async move { (idx, ctx.fetcher.get(&url, &page_options).await) }
        })
        .buffer_unordered(SCHOOL_CONCURRENCY)
        .collect::<Vec<_>>()
        .await
}

/// Read one school page: journal and count it, or record why it was skipped.
pub(super) async fn process_school(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    tally: &mut SchoolTally,
    entry: &IndexEntry,
    result: Result<FetchOutcome, FetchError>,
    observed_on: &str,
) -> CrawlResult<()> {
    let key = format!("WI:{}", entry.org_id);
    let outcome = match result {
        Ok(outcome) => outcome,
        Err(error) => {
            tally.page_failures = tally.page_failures.saturating_add(1);
            report.errors = report.errors.saturating_add(1);
            if tally.page_failures <= 5 {
                report.note(format!("school {key}: {error}"));
            }
            return Ok(());
        }
    };
    if outcome.status == 404 {
        tally.not_found = tally.not_found.saturating_add(1);
        return Ok(());
    }
    if outcome.status != 200 {
        tally.page_failures = tally.page_failures.saturating_add(1);
        report.errors = report.errors.saturating_add(1);
        if tally.page_failures <= 5 {
            report.note(format!("school {key}: HTTP {}", outcome.status));
        }
        return Ok(());
    }

    let page = parse_school_page(&outcome.text());
    let Some(extract) = school_entities(entry, &page, observed_on) else {
        tally.page_failures = tally.page_failures.saturating_add(1);
        report.errors = report.errors.saturating_add(1);
        if tally.page_failures <= 5 {
            report.note(format!("school {key}: page carried no school name"));
        }
        return Ok(());
    };
    record_school(ctx, tally, &key, entry, &page, extract)
}

/// Append the school and its coach rows, count them, and journal both tables.
fn record_school(
    ctx: &AdapterContext<'_>,
    tally: &mut SchoolTally,
    key: &str,
    entry: &IndexEntry,
    page: &SchoolPage,
    extract: SchoolExtract,
) -> CrawlResult<()> {
    ctx.store.append(Table::Schools, &extract.school)?;
    ctx.store.append_many(Table::Coaches, &extract.coaches)?;

    let school_with_email = extract
        .coaches
        .iter()
        .filter(|coach| coach.professional_email.is_some())
        .count();
    tally.with_email = tally.with_email.saturating_add(count(school_with_email));
    tally.coach_rows = tally.coach_rows.saturating_add(extract.coaches.len());
    tally.skipped_coach_rows = tally
        .skipped_coach_rows
        .saturating_add(extract.skipped_coach_rows);
    for role in extract.skipped_admin_roles {
        if !tally.skipped_admin_roles.iter().any(|seen| seen == &role) {
            tally.skipped_admin_roles.push(role);
        }
    }

    let payload = serde_json::json!({
        "org_id": entry.org_id,
        "name": extract.school.name,
        "city": extract.school.city,
        "conference": extract.school.classification,
        "level": page.level,
        "coaches": extract.coaches.len(),
        "coaches_with_email": school_with_email,
    });
    ctx.store.journal_done("wiaa_schools", key, &payload)?;
    ctx.store.journal_done("wiaa_coaches", key, &payload)?;

    tally.processed = tally.processed.saturating_add(1);
    Ok(())
}
