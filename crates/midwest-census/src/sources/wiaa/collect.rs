//! The WIAA directory walk (`collect`), moved verbatim from the flat adapter module.

use crate::model::normalize_name;
use crate::net::{FetchOptions, FetchOutcome};
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};
use std::collections::{BTreeMap, HashSet};

use super::parse::{parse_directory_letter, parse_school_page, IndexEntry};
use super::{count, fetch_options, letters_for, Options, HOST, INDEX_PATH};
use super::{map::school_entities, primitives::meaningful};

// -------------------------------------------------------------------------------------------------
// Collection
// -------------------------------------------------------------------------------------------------

/// Walk the WIAA directory and emit canonical schools plus AD/head-coach rows.
///
/// Resumable: a school page is fetched only when `WI:<orgID>` is absent from the `wiaa_schools`
/// journal, and both journals carry the same `WI:<orgID>` key.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let mut report = AdapterReport::new("wiaa", "schools");
    let observed_on = if options.observed_on.trim().is_empty() {
        ctx.observed_on.clone()
    } else {
        options.observed_on.clone()
    };
    let before = ctx.fetcher.stats().await;

    if !options.states.is_empty()
        && !options
            .states
            .iter()
            .any(|state| state.eq_ignore_ascii_case("WI"))
    {
        let after = ctx.fetcher.stats().await;
        report.requests = after.requests.saturating_sub(before.requests);
        report.note(format!(
            "states {:?} do not include WI; this adapter covers Wisconsin only",
            options.states
        ));
        return Ok(report);
    }

    // -- index: bounded-concurrency fetch per directory letter (N=8) ----------------------------
    const LETTER_CONCURRENCY: usize = 8;
    let letters = letters_for(&options.school_names);
    // Collect (letter_index, result) pairs so we can process in submission order.
    let letter_results: Vec<(usize, Result<FetchOutcome>)> =
        stream::iter(letters.iter().enumerate())
            .map(|(i, letter)| {
                let url = format!("{HOST}{INDEX_PATH}?LetterBtn={letter}");
                let opts = fetch_options(ctx, options);
                async move { (i, ctx.fetcher.get(&url, &opts).await) }
            })
            .buffer_unordered(LETTER_CONCURRENCY)
            .collect::<Vec<_>>()
            .await;

    // Process results in submission order for deterministic error tracking.
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
    if letters_ok == 0 {
        let after = ctx.fetcher.stats().await;
        report.requests = after.requests.saturating_sub(before.requests);
        report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
        anyhow::bail!(
            "WIAA directory index {HOST}{INDEX_PATH} returned no usable letter fragment ({} request(s) attempted): {}",
            letters.len(),
            first_problem.unwrap_or_else(|| "no response".to_string())
        );
    }

    let mut levels: BTreeMap<String, u64> = BTreeMap::new();
    for entry in &index {
        let level = meaningful(&entry.level).unwrap_or_else(|| "unstated".to_string());
        let slot = levels.entry(level).or_insert(0);
        *slot = slot.saturating_add(1);
    }
    let level_summary = levels
        .iter()
        .map(|(level, count)| format!("{level}={count}"))
        .collect::<Vec<_>>()
        .join(", ");

    // -- schools: one request per school ---------------------------------------------------------
    let done = ctx.store.journal_keys("wiaa_schools")?;
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

    let mut processed = 0usize;
    let mut skipped_done = 0usize;
    let mut skipped_filter = 0usize;
    let mut not_found = 0usize;
    let mut page_failures = 0usize;
    let mut coach_rows = 0usize;
    let mut with_email = 0u64;
    let mut skipped_admin_roles: Vec<String> = Vec::new();
    let mut skipped_coach_rows = 0usize;

    // -- schools: bounded-concurrency fetch per school page (N=8) ----------------------------
    const SCHOOL_CONCURRENCY: usize = 8;
    // Phase 1: identify eligible schools (skip already journaled, apply filter).
    let eligible: Vec<(usize, &IndexEntry)> = index
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            if let Some(wanted) = wanted.as_ref() {
                if !wanted.contains(&normalize_name(&entry.name)) {
                    skipped_filter = skipped_filter.saturating_add(1);
                    return false;
                }
            }
            let key = format!("WI:{}", entry.org_id);
            if done.contains(&key) {
                skipped_done = skipped_done.saturating_add(1);
                return false;
            }
            true
        })
        .collect();

    // Phase 2: fetch all school pages in parallel (bounded concurrency N=8).
    // Each fetch is independent — same host, but rate-limited by the fetcher's gate.
    let fetch_results: Vec<(usize, Result<FetchOutcome>)> = stream::iter(eligible)
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
        .await;

    // Phase 3: process results in submission order (deterministic).
    for (idx, result) in fetch_results {
        let Some(entry) = index.get(idx) else {
            continue;
        };
        let key = format!("WI:{}", entry.org_id);

        // Respect limit after collection.
        if let Some(limit) = options.limit {
            if processed >= limit {
                break;
            }
        }

        let outcome = match result {
            Ok(outcome) => outcome,
            Err(error) => {
                page_failures = page_failures.saturating_add(1);
                report.errors = report.errors.saturating_add(1);
                if page_failures <= 5 {
                    report.note(format!("school {key}: {error}"));
                }
                continue;
            }
        };
        if outcome.status == 404 {
            not_found = not_found.saturating_add(1);
            continue;
        }
        if outcome.status != 200 {
            page_failures = page_failures.saturating_add(1);
            report.errors = report.errors.saturating_add(1);
            if page_failures <= 5 {
                report.note(format!("school {key}: HTTP {}", outcome.status));
            }
            continue;
        }

        let page = parse_school_page(&outcome.text());
        let Some(extract) = school_entities(entry, &page, &observed_on) else {
            page_failures = page_failures.saturating_add(1);
            report.errors = report.errors.saturating_add(1);
            if page_failures <= 5 {
                report.note(format!("school {key}: page carried no school name"));
            }
            continue;
        };

        ctx.store
            .append(Table::Schools, &extract.school)
            .with_context(|| format!("writing WIAA school {key}"))?;
        ctx.store
            .append_many(Table::Coaches, &extract.coaches)
            .with_context(|| format!("writing WIAA coaches for {key}"))?;

        let school_with_email = extract
            .coaches
            .iter()
            .filter(|coach| coach.professional_email.is_some())
            .count();
        with_email = with_email.saturating_add(count(school_with_email));
        coach_rows = coach_rows.saturating_add(extract.coaches.len());
        skipped_coach_rows = skipped_coach_rows.saturating_add(extract.skipped_coach_rows);
        for role in extract.skipped_admin_roles {
            if !skipped_admin_roles.iter().any(|seen| seen == &role) {
                skipped_admin_roles.push(role);
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
        ctx.store
            .journal_done("wiaa_schools", &key, &payload)
            .with_context(|| format!("journaling WIAA school {key}"))?;
        ctx.store
            .journal_done("wiaa_coaches", &key, &payload)
            .with_context(|| format!("journaling WIAA coaches for {key}"))?;

        processed = processed.saturating_add(1);
    }

    let after = ctx.fetcher.stats().await;
    report.rows = count(processed);
    report.requests = after.requests.saturating_sub(before.requests);
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    report.with_email = with_email;

    report.note(format!(
        "index: {} letter request(s), {} schools listed ({level_summary})",
        letters.len(),
        index.len()
    ));
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
    report.note(format!(
        "skipped {skipped_done} already journaled, {skipped_filter} outside the requested school names, {not_found} HTTP 404, {page_failures} page failures"
    ));
    report.note(format!(
        "not imported: {skipped_coach_rows} coach-table rows outside TF/XC or without a coaching role, {} non-director administration roles",
        count(skipped_admin_roles.len())
    ));
    if !skipped_admin_roles.is_empty() {
        let mut sorted = skipped_admin_roles;
        sorted.sort_unstable();
        report.note(format!(
            "non-director admin roles seen: {}",
            sorted.join(", ")
        ));
    }
    if let Some(limit) = options.limit {
        report.note(format!("limit applied: {limit} school(s)"));
    }
    Ok(report)
}
