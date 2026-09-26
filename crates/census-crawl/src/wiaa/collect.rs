//! The WIAA directory walk (`collect`), moved verbatim from the flat adapter module.

use crate::net::{FetchError, FetchOutcome, FetchStats};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::UsJurisdiction;
use futures::stream::{self, StreamExt};
use std::collections::{BTreeMap, HashSet};

use super::parse::{parse_directory_letter, IndexEntry};
use super::primitives::meaningful;
use super::{count, fetch_options, letters_for, Options, HOST, INDEX_PATH};

#[path = "collect_schools.rs"]
mod collect_schools;

use collect_schools::{plan_schools, process_school, SchoolTally};


/// Walk the WIAA directory and emit canonical schools plus AD/head-coach rows.
///
/// Resumable: a school page is fetched only when `WI:<orgID>` is absent from the `wiaa_schools`
/// journal, and both journals carry the same `WI:<orgID>` key.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("wiaa", "schools");
    let observed_on = if options.observed_on.trim().is_empty() {
        ctx.observed_on.clone()
    } else {
        options.observed_on.clone()
    };
    let before = ctx.fetcher.stats().await;

    if !options.states.is_empty() && !options.states.contains(&UsJurisdiction::Wisconsin) {
        let after = ctx.fetcher.stats().await;
        report.requests = after.requests.saturating_sub(before.requests);
        let codes: Vec<&str> = options.states.iter().map(|state| state.code()).collect();
        report.note(format!(
            "states {codes:?} do not include WI; this adapter covers Wisconsin only"
        ));
        return Ok(report);
    }

    let LetterScan { index, letters } = scan_index(ctx, options, &mut report, &before).await?;
    let level_summary = summarize_levels(&index);

    let done = ctx.store.journal_keys("wiaa_schools")?;
    let mut tally = SchoolTally::default();
    let fetch_results = plan_schools(ctx, options, &index, &done, &mut tally).await;

    for (idx, result) in fetch_results {
        let Some(entry) = index.get(idx) else {
            continue;
        };

        if let Some(limit) = options.limit {
            if tally.processed >= limit {
                break;
            }
        }

        process_school(ctx, &mut report, &mut tally, entry, result, &observed_on).await?;
    }

    let after = ctx.fetcher.stats().await;
    report.rows = count(tally.processed);
    report.requests = after.requests.saturating_sub(before.requests);
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    report.with_email = tally.with_email;

    note_index(&mut report, letters, index.len(), &level_summary);
    note_written(&mut report, &tally);
    note_skips(&mut report, tally, options.limit);
    Ok(report)
}

/// The directory index: its entries (deduplicated by org id) and how many letters were walked.
struct LetterScan {
    index: Vec<IndexEntry>,
    letters: usize,
}

/// Walk the directory letters and build the school index they list.
async fn scan_index(
    ctx: &AdapterContext<'_>,
    options: &Options,
    report: &mut AdapterReport,
    before: &FetchStats,
) -> CrawlResult<LetterScan> {
    const LETTER_CONCURRENCY: usize = crate::CONCURRENCY_BOUND;
    let letters = letters_for(&options.school_names);
    let letter_results: Vec<(usize, Result<FetchOutcome, FetchError>)> =
        stream::iter(letters.iter().copied().enumerate())
            .map(|(i, letter)| {
                let url = format!("{HOST}{INDEX_PATH}?LetterBtn={letter}");
                let opts = fetch_options(ctx, options);
                async move { (i, ctx.fetcher.get(&url, &opts).await) }
            })
            .buffer_unordered(LETTER_CONCURRENCY)
            .collect::<Vec<_>>()
            .await;

    let (index, letters_ok, first_problem) = absorb_letters(&letters, letter_results, report);
    if letters_ok == 0 {
        let after = ctx.fetcher.stats().await;
        report.requests = after.requests.saturating_sub(before.requests);
        report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
        return Err(CrawlError::Schema {
            url: format!("{HOST}{INDEX_PATH}"),
            detail: format!(
                "returned no usable letter fragment ({} request(s) attempted): {}",
                letters.len(),
                first_problem.unwrap_or_else(|| "no response".to_string())
            ),
        });
    }

    Ok(LetterScan {
        index,
        letters: letters.len(),
    })
}

/// Read the letter fragments in submission order: parse rows, dedupe ids, track the first problem.
fn absorb_letters(
    letters: &[char],
    letter_results: Vec<(usize, Result<FetchOutcome, FetchError>)>,
    report: &mut AdapterReport,
) -> (Vec<IndexEntry>, usize, Option<String>) {
    let mut index: Vec<IndexEntry> = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut letters_ok = 0usize;
    let mut first_problem: Option<String> = None;
    for (i, result) in letter_results {
        let Some(letter) = letters.get(i).copied() else {
            continue;
        };
        match result {
            Ok(outcome) if outcome.status == 200 => {
                letters_ok = letters_ok.saturating_add(1);
                for entry in parse_directory_letter(&outcome.text()) {
                    if seen_ids.insert(entry.org_id.clone()) {
                        index.push(entry);
                    }
                }
            }
            Ok(outcome) => {
                report.errors = report.errors.saturating_add(1);
                let problem = format!("letter {letter} returned HTTP {}", outcome.status);
                report.note(problem.clone());
                if first_problem.is_none() {
                    first_problem = Some(problem);
                }
            }
            Err(error) => {
                report.errors = report.errors.saturating_add(1);
                let problem = format!("letter {letter}: {error}");
                report.note(problem.clone());
                if first_problem.is_none() {
                    first_problem = Some(problem);
                }
            }
        }
    }
    (index, letters_ok, first_problem)
}

/// Count the listed schools per published level, for the index note.
fn summarize_levels(index: &[IndexEntry]) -> String {
    let mut levels: BTreeMap<String, u64> = BTreeMap::new();
    for entry in index {
        let level = meaningful(&entry.level).unwrap_or_else(|| "unstated".to_string());
        let slot = levels.entry(level).or_insert(0);
        *slot = slot.saturating_add(1);
    }
    levels
        .iter()
        .map(|(level, count)| format!("{level}={count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Note how many directory letters were requested and how many schools they listed.
fn note_index(report: &mut AdapterReport, letters: usize, index_len: usize, level_summary: &str) {
    report.note(format!(
        "index: {} letter request(s), {} schools listed ({level_summary})",
        letters, index_len
    ));
}

/// Note how many schools and coach rows were written, and the published email fill rate.
fn note_written(report: &mut AdapterReport, tally: &SchoolTally) {
    let processed = tally.processed;
    let coach_rows = tally.coach_rows;
    let with_email = tally.with_email;
    report.note(format!(
        "wrote {processed} schools and {coach_rows} AD/head-coach rows to the store"
    ));
    report.note(if coach_rows == 0 {
        "no coach or athletic-director row was published on the fetched pages".to_string()
    } else if with_email == 0 {
        "no email published on this surface".to_string()
    } else {
        let percent = with_email
            .checked_mul(100)
            .and_then(|scaled| scaled.checked_div(count(coach_rows)))
            .unwrap_or(0);
        format!("published coach/AD email fill rate: {with_email}/{coach_rows} rows ({percent}%)")
    });
}

/// Note what the walk skipped, which administration roles it refused, and the applied limit.
fn note_skips(report: &mut AdapterReport, tally: SchoolTally, limit: Option<usize>) {
    let skipped_done = tally.skipped_done;
    let skipped_filter = tally.skipped_filter;
    let not_found = tally.not_found;
    let page_failures = tally.page_failures;
    let skipped_coach_rows = tally.skipped_coach_rows;
    report.note(format!(
        "skipped {skipped_done} already journaled, {skipped_filter} outside the requested school names, {not_found} HTTP 404, {page_failures} page failures"
    ));
    report.note(format!(
        "not imported: {skipped_coach_rows} coach-table rows outside TF/XC or without a coaching role, {} non-director administration roles",
        count(tally.skipped_admin_roles.len())
    ));
    if !tally.skipped_admin_roles.is_empty() {
        let mut sorted = tally.skipped_admin_roles;
        sorted.sort_unstable();
        report.note(format!(
            "non-director admin roles seen: {}",
            sorted.join(", ")
        ));
    }
    if let Some(limit) = limit {
        report.note(format!("limit applied: {limit} school(s)"));
    }
}
