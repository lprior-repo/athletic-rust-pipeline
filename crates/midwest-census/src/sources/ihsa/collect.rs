//! Collect orchestration: fetch the schools list once, then walk one `staff2` request per
//! school, journalling progress so a re-run resumes without re-processing.

use super::map::{parse_coach, parse_school, reveal_address_for};
use super::parse::{parse_email, parse_schools, parse_staff};
use super::{Options, IHSA_API};
use crate::model::{CanonicalCoach, CanonicalSchool, Evidence, SourceRef};
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::{Context, Result};
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Collect
// ---------------------------------------------------------------------------

/// Collect this provider's Illinois schools and coach/AD contacts into the canonical store.
///
/// Strategy:
/// 1. Fetch `https://api.ihsa.org/v1/schools` — one request returns all 828 Illinois member schools.
/// 2. For each school, fetch `https://api.ihsa.org/v1/schools/{SchoolID}/staff2` — per-school staff.
/// 3. Parse schools into canonical schools, staff into coaches (ADs + head/assistant coaches).
/// 4. Journal progress per school so a re-run resumes without re-processing.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let mut report = AdapterReport::new("ihsa", "schools");
    report.unit = "schools".to_string();

    // Snapshot fetcher stats before work.
    let before = ctx.fetcher.stats().await;

    // Load the resume set (keys already journalled in this phase).
    let done_keys: HashSet<String> = ctx.store.journal_keys("ihsa_schools")?;

    let limit = options.limit;
    // Tally counters saturate: they feed diagnostics only, so an impossible overflow floors at
    // `usize::MAX` instead of panicking or wrapping silently.
    let mut processed = 0usize;
    let mut skipped = 0usize;
    let mut schools: Vec<CanonicalSchool> = Vec::new();
    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    // PersonID → revealed address within the current school (a person can hold several titles).
    let mut revealed_emails: HashMap<i64, Option<String>> = HashMap::new();

    // ── Step 1: Fetch the schools list ───────────────────────────────────
    let schools_url = format!("{IHSA_API}/v1/schools");
    let outcome = match ctx.fetcher.get(&schools_url, &ctx.fetch_options()).await {
        Ok(o) => o,
        Err(e) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!("failed to fetch IHSA schools: {e}"));
            return Ok(report);
        }
    };

    let records = parse_schools(&outcome.text())?;
    report.requests = report.requests.saturating_add(1);
    if outcome.from_cache {
        report.from_cache = report.from_cache.saturating_add(1);
    }

    // ── Step 2: For each school, fetch staff and parse ───────────────────
    for record in &records {
        // Honour limit.
        if let Some(max) = limit {
            if processed >= max {
                break;
            }
        }

        // Skip already-processed schools (resume support).
        let journal_key = format!("IL:{}", record.school_id);
        if done_keys.contains(&journal_key) {
            skipped = skipped.saturating_add(1);
            continue;
        }
        revealed_emails.clear();

        // Parse school.
        let (school, school_id) = match parse_school(record, &schools_url, &options.observed_on) {
            Some(pair) => pair,
            None => continue,
        };
        schools.push(school);

        // Fetch staff for this school.
        let staff_url = format!("{IHSA_API}/v1/schools/{}/staff2", record.school_id);
        let staff_outcome = match ctx.fetcher.get(&staff_url, &ctx.fetch_options()).await {
            Ok(o) => o,
            Err(e) => {
                report.errors = report.errors.saturating_add(1);
                report.note(format!(
                    "failed to fetch staff for school {}: {e}",
                    record.school_id
                ));
                ctx.store
                    .journal_done(
                        "ihsa_schools",
                        &journal_key,
                        &serde_json::json!({
                            "school_id": record.school_id,
                            "name": record.name_formal,
                            "error": e.to_string(),
                        }),
                    )
                    .context("journaling ihsa school progress")?;
                processed = processed.saturating_add(1);
                continue;
            }
        };
        report.requests = report.requests.saturating_add(1);
        if staff_outcome.from_cache {
            report.from_cache = report.from_cache.saturating_add(1);
        }

        // Parse staff and emit coaches.
        let staff = match parse_staff(&staff_outcome.text()) {
            Ok(env) => env,
            Err(e) => {
                report.errors = report.errors.saturating_add(1);
                report.note(format!(
                    "failed to parse staff for school {}: {e}",
                    record.school_id
                ));
                ctx.store
                    .journal_done(
                        "ihsa_schools",
                        &journal_key,
                        &serde_json::json!({
                            "school_id": record.school_id,
                            "name": record.name_formal,
                            "parse_error": e.to_string(),
                        }),
                    )
                    .context("journaling ihsa school progress")?;
                processed = processed.saturating_add(1);
                continue;
            }
        };

        for person in &staff {
            if let Some(mut coach) =
                parse_coach(person, &school_id, &staff_url, &options.observed_on)
            {
                // `staff2` carries a `HasEmail` flag but never the address itself; the address is
                // behind `GET /v1/schools/{id}/staff/{PersonID}/email` (the "Show email" button).
                // One reveal per person, and only for the roles the census ships: the recruiting
                // projection reads TF/XC head coaches and ADs, so a bowling coach's address is not
                // worth a request.
                if person.has_email == Some(true) && reveal_address_for(&coach) {
                    let email_url = format!(
                        "{IHSA_API}/v1/schools/{}/staff/{}/email",
                        record.school_id, person.person_id
                    );
                    let revealed = match revealed_emails.entry(person.person_id) {
                        std::collections::hash_map::Entry::Occupied(slot) => slot.get().clone(),
                        std::collections::hash_map::Entry::Vacant(slot) => {
                            let value =
                                match ctx.fetcher.get(&email_url, &ctx.fetch_options()).await {
                                    Ok(outcome) => {
                                        report.requests = report.requests.saturating_add(1);
                                        if outcome.from_cache {
                                            report.from_cache = report.from_cache.saturating_add(1);
                                        }
                                        parse_email(&outcome.text())
                                    }
                                    Err(e) => {
                                        report.errors = report.errors.saturating_add(1);
                                        report.note(format!(
                                            "email reveal failed for person {}: {e}",
                                            person.person_id
                                        ));
                                        None
                                    }
                                };
                            slot.insert(value.clone());
                            value
                        }
                    };
                    if let Some(address) = revealed {
                        coach.professional_email = Some(address);
                        coach.evidence.push(Evidence::parsed(
                            SourceRef::new("ihsa", Some(email_url)),
                            &options.observed_on,
                        ));
                    }
                }
                if coach.professional_email.is_some() {
                    report.with_email = report.with_email.saturating_add(1);
                }
                coaches.push(coach);
            }
        }

        // Journal this school as done.
        ctx.store
            .journal_done(
                "ihsa_schools",
                &journal_key,
                &serde_json::json!({
                    "school_id": record.school_id,
                    "name": record.name_formal,
                }),
            )
            .context("journaling ihsa school progress")?;

        processed = processed.saturating_add(1);
    }

    // Append all schools and coaches to the store.
    if !schools.is_empty() {
        ctx.store
            .append_many(Table::Schools, &schools)
            .context("writing ihsa schools")?;
    }
    if !coaches.is_empty() {
        ctx.store
            .append_many(Table::Coaches, &coaches)
            .context("writing ihsa coaches")?;
    }

    // Finalize stats and counts.
    let after = ctx.fetcher.stats().await;
    let delta_requests = after.requests.saturating_sub(before.requests);
    report.rows = u64::try_from(processed).context("ihsa school count exceeds u64")?;
    report.requests = delta_requests;
    report.note(format!(
        "fetched {} Illinois schools from IHSA; {} already done",
        processed, skipped
    ));

    Ok(report)
}
