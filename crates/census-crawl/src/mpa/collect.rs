//! The adapter body: resolve the school directory, walk each school's staff page, append and journal.
//!
//! Network and store access live here and nowhere else in this module.

use super::map::{school_entities, ParsedSchool, SchoolExtract};
use super::pages::parse_directory;
use super::{Options, ASSOCIATION, HOST_WWW};
use crate::net::{FetchOptions, FetchStats};
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::normalize_name;
use census_domain::model::SourceNamespace;
use census_domain::UsJurisdiction;
use census_store::Table;

// ── Collection ─────────────────────────────────────────────────────────────

/// Collect MPA schools and coaches.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("mpa", "schools");
    let observed_on = if options.observed_on.trim().is_empty() {
        ctx.observed_on.clone()
    } else {
        options.observed_on.clone()
    };
    let before = ctx.fetcher.stats().await;

    // Restrict to ME state
    if !options.states.is_empty() && !options.states.contains(&UsJurisdiction::Maine) {
        record_spend(ctx, &mut report, &before).await;
        let codes: Vec<&str> = options.states.iter().map(|state| state.code()).collect();
        report.note(format!(
            "states {codes:?} do not include ME; this adapter covers Maine only"
        ));
        return Ok(report);
    }

    // Fetch the directory page
    let dir_url = format!("{HOST_WWW}/SchoolPages/School.aspx");
    let dir_html = match ctx.fetcher.get(&dir_url, &ctx.fetch_options()).await {
        Ok(outcome) if outcome.status == 200 => outcome.text(),
        Ok(outcome) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!("directory returned HTTP {}", outcome.status));
            record_spend(ctx, &mut report, &before).await;
            return Ok(report);
        }
        Err(e) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!("directory fetch failed: {e}"));
            record_spend(ctx, &mut report, &before).await;
            return Ok(report);
        }
    };

    let entries = parse_directory(&dir_html);
    let to_process = resolve_schools(&entries, options);
    let mut tally = Tally::default();

    for entry in &to_process {
        process_school(ctx, entry, &observed_on, &mut report, &mut tally).await?;
    }

    record_spend(ctx, &mut report, &before).await;

    report.note(format!(
        "processed {} of {} requested schools ({} fetch_failures, {} coach_rows)",
        tally.processed,
        to_process.len(),
        tally.fetch_failures,
        tally.coach_rows,
    ));

    Ok(report)
}

/// Fold the fetcher's counters since `before` into the report.
///
/// Every exit from [`collect`] records the same two numbers, including the ones that return before
/// the walk: a run that stopped at the directory page still spent its request, and a report that
/// omitted it would look free.
async fn record_spend(ctx: &AdapterContext<'_>, report: &mut AdapterReport, before: &FetchStats) {
    let after = ctx.fetcher.stats().await;
    report.requests = after.requests.saturating_sub(before.requests);
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
}

/// Counters for one collect run.
#[derive(Default)]
struct Tally {
    processed: usize,
    fetch_failures: usize,
    coach_rows: usize,
}

/// Resolve which schools to process, applying limit and name filters.
fn resolve_schools(entries: &[super::pages::SchoolEntry], options: &Options) -> Vec<ParsedSchool> {
    let wanted: Option<std::collections::HashSet<String>> = if options.school_names.is_empty() {
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

    entries
        .iter()
        .filter_map(|e| {
            if let Some(ref wanted) = wanted {
                if !wanted.contains(&normalize_name(&e.name)) {
                    return None;
                }
            }
            Some(ParsedSchool {
                name: e.name.clone(),
                school_id: e.school_id.clone(),
            })
        })
        .take(options.limit.unwrap_or(usize::MAX))
        .collect()
}

/// Fetch one school's staff page, parse and emit.
async fn process_school(
    ctx: &AdapterContext<'_>,
    entry: &ParsedSchool,
    observed_on: &str,
    report: &mut AdapterReport,
    tally: &mut Tally,
) -> CrawlResult<()> {
    let staff_url = format!(
        "{HOST_WWW}/SchoolPages/School.aspx?SchoolID={}&tab=staff",
        entry.school_id
    );
    let staff_html = match ctx
        .fetcher
        .get(
            &staff_url,
            &FetchOptions {
                allow_not_found: true,
                ..ctx.fetch_options()
            },
        )
        .await
    {
        Ok(outcome) if outcome.status == 200 => outcome.text(),
        Ok(outcome) => {
            report.note(format!(
                "school {} (ID {}) returned HTTP {}",
                entry.name, entry.school_id, outcome.status
            ));
            return Ok(());
        }
        Err(e) => {
            tally.fetch_failures = tally.fetch_failures.saturating_add(1);
            report.note(format!("school {}: {}", entry.name, e));
            return Ok(());
        }
    };

    let staff_rows = super::pages::parse_staff_table(&staff_html);
    let extract = school_entities(entry, &staff_rows, observed_on);

    emit_school(ctx, report, &extract, tally)?;
    tally.processed = tally.processed.saturating_add(1);
    Ok(())
}

/// Append and journal one school's rows.
fn emit_school(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    extract: &SchoolExtract,
    tally: &mut Tally,
) -> CrawlResult<()> {
    let key = &extract.source_school_id;
    let mut batch = ctx.write_batch();
    batch.append_many(Table::Schools, std::slice::from_ref(&extract.school))?;
    ctx.observe_school(
        &SourceNamespace::association_school(ASSOCIATION),
        &extract.school,
    )?;
    report.rows = report.rows.saturating_add(1);

    tally.coach_rows = tally.coach_rows.saturating_add(extract.coaches.len());
    batch.append_many(Table::Coaches, &extract.coaches)?;

    batch.journal_done(
        "mpa_schools",
        key,
        &serde_json::json!({
            "school_name": extract.school.name,
            "coaches": extract.coaches.len(),
        }),
    )?;
    for coach in &extract.coaches {
        batch.journal_done(
            "mpa_coaches",
            key,
            &serde_json::json!({
                "coach_name": coach.name,
                "sport": format!("{:?}", coach.sport),
                "gender": format!("{:?}", coach.gender),
            }),
        )?;
    }
    batch.commit()?;
    Ok(())
}
