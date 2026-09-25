//! The adapter body: resolve the school list, walk each school's pages, append and journal.
//!
//! Network and store access live here and nowhere else in this module.

use super::map::{school_entities, SchoolExtract, SearchResult};
use super::pages::parse_ad_page;
use super::{Options, ASSOCIATION};
use crate::net::FetchOptions;
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::SourceNamespace;
use census_domain::UsJurisdiction;
use census_store::Table;

mod search;

use search::resolve_schools;

// ── Collection ─────────────────────────────────────────────────────────────

/// Collect OHSAA schools and coaches.
///
/// Resumable: a school page is fetched only when `OH:<ohsaaId>` is absent from
/// the `ohsaa_schools` journal. Both journals carry the same key.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("ohsaa", "schools");
    let observed_on = if options.observed_on.trim().is_empty() {
        ctx.observed_on.clone()
    } else {
        options.observed_on.clone()
    };
    let before = ctx.fetcher.stats().await;

    // Restrict to OH state
    if !options.states.is_empty() && !options.states.contains(&UsJurisdiction::Ohio) {
        let after = ctx.fetcher.stats().await;
        report.requests = after.requests.saturating_sub(before.requests);
        report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
        let codes: Vec<&str> = options.states.iter().map(|state| state.code()).collect();
        report.note(format!(
            "states {codes:?} do not include OH; this adapter covers Ohio only"
        ));
        return Ok(report);
    }

    let to_process = resolve_schools(ctx, options, &mut report).await?;
    let skipped_done = 0usize;
    let mut tally = Tally::default();

    for sr in &to_process {
        process_school(ctx, sr, &observed_on, &mut report, &mut tally).await?;
    }

    let after = ctx.fetcher.stats().await;
    report.requests = after.requests.saturating_sub(before.requests);
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    report.with_email = tally.with_email;

    report.note(format!(
        "processed {} of {} requested schools ({} skipped_done, {} not_found, {} fetch_failures, {} coach_rows, {} office_roles_skipped)",
        tally.processed, to_process.len(), skipped_done, tally.not_found, tally.fetch_failures, tally.coach_rows, tally.office_roles_skipped
    ));

    Ok(report)
}

/// Counters for one `collect` run, kept in a struct so each stage can update them.
#[derive(Default)]
struct Tally {
    processed: usize,
    not_found: usize,
    fetch_failures: usize,
    coach_rows: usize,
    with_email: u64,
    office_roles_skipped: usize,
}

/// Fetch options for a school page that may legitimately be absent.
fn page_fetch_options(ctx: &AdapterContext<'_>) -> FetchOptions {
    FetchOptions {
        allow_not_found: true,
        ..ctx.fetch_options()
    }
}

/// The school's sports and AD pages, or `None` when the sports page could not be retrieved.
async fn fetch_pages(
    ctx: &AdapterContext<'_>,
    sr: &SearchResult,
    report: &mut AdapterReport,
    tally: &mut Tally,
) -> Option<(String, String)> {
    let sports_url = sr.sports_url();
    let sports_html = match ctx.fetcher.get(&sports_url, &page_fetch_options(ctx)).await {
        Ok(outcome) if outcome.status == 200 => outcome.text(),
        Ok(outcome) if outcome.status == 404 => {
            tally.not_found = tally.not_found.saturating_add(1);
            report.note(format!(
                "school {} (ID {}) returned 404",
                sr.name, sr.ohsaa_id
            ));
            return None;
        }
        Ok(outcome) => {
            tally.fetch_failures = tally.fetch_failures.saturating_add(1);
            report.note(format!(
                "school {} sports returned HTTP {}",
                sr.name, outcome.status
            ));
            return None;
        }
        Err(e) => {
            tally.fetch_failures = tally.fetch_failures.saturating_add(1);
            report.note(format!("school {} sports: {}", sr.name, e));
            return None;
        }
    };

    let ad_url = sr.ad_url();
    let ad_html = match ctx.fetcher.get(&ad_url, &page_fetch_options(ctx)).await {
        Ok(outcome) if outcome.status == 200 => outcome.text(),
        Ok(outcome) => {
            report.note(format!(
                "school {} AD page returned HTTP {}",
                sr.name, outcome.status
            ));
            String::new()
        }
        Err(e) => {
            report.note(format!("school {} AD page: {}", sr.name, e));
            String::new()
        }
    };
    Some((sports_html, ad_html))
}

/// Parse one school's pages, append its rows, journal them and tally the run.
async fn process_school(
    ctx: &AdapterContext<'_>,
    sr: &SearchResult,
    observed_on: &str,
    report: &mut AdapterReport,
    tally: &mut Tally,
) -> CrawlResult<()> {
    let Some((sports_html, ad_html)) = fetch_pages(ctx, sr, report, tally).await else {
        return Ok(());
    };
    let extract = school_entities(sr, &sports_html, &ad_html, observed_on);

    // Count office roles skipped
    let ad = parse_ad_page(&ad_html);
    tally.office_roles_skipped = tally
        .office_roles_skipped
        .saturating_add(ad.office_roles.len());

    emit_school(ctx, sr, report, &extract, tally)?;
    tally.processed = tally.processed.saturating_add(1);
    Ok(())
}

/// Append and journal one school's rows, tallying its coach and email counts.
fn emit_school(
    ctx: &AdapterContext<'_>,
    sr: &SearchResult,
    report: &mut AdapterReport,
    extract: &SchoolExtract,
    tally: &mut Tally,
) -> CrawlResult<()> {
    let school_key = format!("OH:{}", sr.ohsaa_id);
    // One page: the school, every coach row and every journal entry commit together. The coach rows go
    // in one append rather than one call per coach, and the entries are buffered in the same page, so
    // the unit is one durability boundary instead of one per row. The school observation stays a
    // direct write, the way every school arm writes it.
    let mut batch = ctx.store.write_batch();
    batch.append_many(Table::Schools, std::slice::from_ref(&extract.school))?;
    ctx.observe_school(
        &SourceNamespace::association_school(ASSOCIATION),
        &extract.school,
    )?;
    report.rows = report.rows.saturating_add(1);
    let mut coach_emails = 0u64;
    for coach in &extract.coaches {
        tally.coach_rows = tally.coach_rows.saturating_add(1);
        if coach.professional_email.is_some() || coach.personal_email.is_some() {
            coach_emails = coach_emails.saturating_add(1);
        }
    }
    batch.append_many(Table::Coaches, &extract.coaches)?;
    tally.with_email = tally.with_email.saturating_add(coach_emails);

    batch.journal_done(
        "ohsaa_schools",
        &school_key,
        &serde_json::json!({
            "ohsaa_id": sr.ohsaa_id,
            "city": sr.city,
            "coaches": extract.coaches.len(),
        }),
    )?;
    for coach in &extract.coaches {
        batch.journal_done(
            "ohsaa_coaches",
            &school_key,
            &serde_json::json!({
                "coach_name": coach.name,
                "sport": format!("{:?}", coach.sport),
                "gender": format!("{:?}", coach.gender),
                "role": format!("{:?}", coach.role),
                "email": coach.professional_email.is_some() || coach.personal_email.is_some(),
            }),
        )?;
    }
    batch.commit()?;
    Ok(())
}
