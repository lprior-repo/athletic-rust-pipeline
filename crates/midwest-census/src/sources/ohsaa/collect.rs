//! The adapter body: resolve the school list, walk each school's pages, append and journal.
//!
//! Network and store access live here and nowhere else in this module.

use super::map::{school_entities, SearchResult};
use super::pages::parse_ad_page;
use super::parse::resolve_school_name;
use super::{Options, HOST, SEARCH_PATH};
use crate::model::SourceNamespace;
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::Result;
use std::collections::HashSet;

// ── Collection ─────────────────────────────────────────────────────────────

/// Collect OHSAA schools and coaches.
///
/// Resumable: a school page is fetched only when `OH:<ohsaaId>` is absent from
/// the `ohsaa_schools` journal. Both journals carry the same key.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let mut report = AdapterReport::new("ohsaa", "schools");
    let observed_on = if options.observed_on.trim().is_empty() {
        ctx.observed_on.clone()
    } else {
        options.observed_on.clone()
    };
    let before = ctx.fetcher.stats().await;

    // Restrict to OH state
    if !options.states.is_empty() && !options.states.iter().any(|s| s.eq_ignore_ascii_case("OH")) {
        let after = ctx.fetcher.stats().await;
        report.requests = after.requests.saturating_sub(before.requests);
        report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
        report.note(format!(
            "states {:?} do not include OH; this adapter covers Ohio only",
            options.states
        ));
        return Ok(report);
    }

    // Resolve schools
    let mut to_process: Vec<SearchResult> = if !options.school_names.is_empty() {
        let mut results = Vec::new();
        for name in &options.school_names {
            let url = format!("{HOST}{SEARCH_PATH}?Name={}", url_encode(name));
            let mut notes: Vec<String> = Vec::new();
            match ctx.fetcher.get(&url, &ctx.fetch_options()).await {
                Ok(outcome) if outcome.status == 200 => {
                    if let Some(sr) = resolve_school_name(&outcome.text(), name, &mut notes) {
                        results.push(sr);
                    } else {
                        report.note(format!("no school found for \"{}\"", name));
                    }
                    for n in notes {
                        report.note(n);
                    }
                }
                Ok(outcome) => {
                    report.errors = report.errors.saturating_add(1);
                    report.note(format!(
                        "search \"{}\" returned HTTP {}",
                        name, outcome.status
                    ));
                }
                Err(e) => {
                    report.errors = report.errors.saturating_add(1);
                    report.note(format!("search \"{}\": {}", name, e));
                }
            }
        }
        results
    } else {
        // Read existing OH schools from schools.jsonl
        let existing = crate::report::read_rows::<crate::model::CanonicalSchool>(
            &ctx.store.out_dir().join("schools.jsonl"),
        )?;
        existing
            .into_iter()
            .filter(|s| {
                matches!(
                    &s.association,
                    Some(ref a) if a.as_str() == "ohsaa"
                )
            })
            .map(|s| SearchResult {
                name: s.name,
                city: s.city.clone().unwrap_or_default(),
                ohsaa_id: s
                    .source_identities
                    .iter()
                    .find_map(|si| match &si.namespace {
                        SourceNamespace::AssociationSchool { .. } => Some(&si.id),
                        _ => None,
                    })
                    .cloned()
                    .unwrap_or_default(),
            })
            .collect()
    };

    // Deduplicate by ohsaaId
    let mut seen_ids: HashSet<String> = HashSet::new();
    to_process.retain(|sr| seen_ids.insert(sr.ohsaa_id.clone()));

    // Check journal for already-processed schools
    let done = ctx.store.journal_keys("ohsaa_schools")?;
    to_process.retain(|sr| !done.contains(&format!("OH:{}", sr.ohsaa_id)));

    // Apply limit
    if let Some(limit) = options.limit {
        to_process.truncate(limit);
    }

    let total_schools = to_process.len();

    let mut processed = 0usize;
    let skipped_done = 0usize;
    let mut not_found = 0usize;
    let mut fetch_failures = 0usize;
    let mut coach_rows = 0usize;
    let mut with_email = 0u64;
    let mut office_roles_skipped = 0usize;

    for sr in &to_process {
        let sports_url = sr.sports_url();
        let ad_url = sr.ad_url();
        let school_key = format!("OH:{}", sr.ohsaa_id);

        // Fetch sports page
        let sports_result = ctx
            .fetcher
            .get(
                &sports_url,
                &crate::net::FetchOptions {
                    allow_not_found: true,
                    ..ctx.fetch_options()
                },
            )
            .await;

        let sports_html = match sports_result {
            Ok(outcome) if outcome.status == 200 => outcome.text(),
            Ok(outcome) if outcome.status == 404 => {
                not_found = not_found.saturating_add(1);
                report.note(format!(
                    "school {} (ID {}) returned 404",
                    sr.name, sr.ohsaa_id
                ));
                continue;
            }
            Ok(outcome) => {
                fetch_failures = fetch_failures.saturating_add(1);
                report.note(format!(
                    "school {} sports returned HTTP {}",
                    sr.name, outcome.status
                ));
                continue;
            }
            Err(e) => {
                fetch_failures = fetch_failures.saturating_add(1);
                report.note(format!("school {} sports: {}", sr.name, e));
                continue;
            }
        };

        // Fetch AD page
        let ad_result = ctx
            .fetcher
            .get(
                &ad_url,
                &crate::net::FetchOptions {
                    allow_not_found: true,
                    ..ctx.fetch_options()
                },
            )
            .await;

        let ad_html = match ad_result {
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

        // Parse and emit entities
        let extract = school_entities(sr, &sports_html, &ad_html, &observed_on);

        // Count office roles skipped
        let ad = parse_ad_page(&ad_html);
        office_roles_skipped = office_roles_skipped.saturating_add(ad.office_roles.len());

        // Append school
        ctx.store.append(Table::Schools, &extract.school)?;
        report.rows = report.rows.saturating_add(1);

        // Append coaches
        let mut coach_emails = 0u64;
        for coach in &extract.coaches {
            ctx.store.append(Table::Coaches, coach)?;
            coach_rows = coach_rows.saturating_add(1);
            if coach.professional_email.is_some() {
                coach_emails = coach_emails.saturating_add(1);
            }
        }
        with_email = with_email.saturating_add(coach_emails);

        // Journal both schools and coaches
        ctx.store.journal_done(
            "ohsaa_schools",
            &school_key,
            &serde_json::json!({
                "ohsaa_id": sr.ohsaa_id,
                "city": sr.city,
                "coaches": extract.coaches.len(),
            }),
        )?;

        // Journal each coach under the same key
        for coach in &extract.coaches {
            ctx.store.journal_done(
                "ohsaa_coaches",
                &school_key,
                &serde_json::json!({
                    "coach_name": coach.name,
                    "sport": format!("{:?}", coach.sport),
                    "gender": format!("{:?}", coach.gender),
                    "role": format!("{:?}", coach.role),
                    "email": coach.professional_email.is_some(),
                }),
            )?;
        }

        processed = processed.saturating_add(1);
    }

    let after = ctx.fetcher.stats().await;
    report.requests = after.requests.saturating_sub(before.requests);
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    report.with_email = with_email;

    report.note(format!(
        "processed {} of {} requested schools ({} skipped_done, {} not_found, {} fetch_failures, {} coach_rows, {} office_roles_skipped)",
        processed, total_schools, skipped_done, not_found, fetch_failures, coach_rows, office_roles_skipped
    ));

    Ok(report)
}

/// URL-encode a school name for the search query parameter.
fn url_encode(value: &str) -> String {
    let mut result = String::new();
    for ch in value.chars() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(ch),
            ' ' => result.push_str("%20"),
            _ => {
                for byte in ch.to_string().bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    result
}
