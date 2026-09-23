//! Collect orchestration: fetch the schools list once, then walk one `staff2` request per school,
//! appending each school's rows before journalling it so a re-run resumes past it.

use super::map::{parse_coach, parse_school, reveal_address_for};
use super::parse::{parse_email, parse_schools, parse_staff, SchoolRecord, StaffPerson};
use super::{Options, IHSA_API};
use crate::sources::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{CanonicalCoach, Evidence, SchoolId, SourceRef};
use census_store::Table;
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
/// 4. Append each school and its coach rows, then journal that school, so a re-run resumes past
///    every school whose rows are already in the store.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("ihsa", "schools");
    report.unit = "schools".to_string();

    let before = ctx.fetcher.stats().await;
    let done_keys: HashSet<String> = ctx.store.journal_keys("ihsa_schools")?;

    let schools_url = format!("{IHSA_API}/v1/schools");
    let Some(records) = fetch_school_records(ctx, &mut report, &schools_url).await? else {
        return Ok(report);
    };

    let mut run = IhsaRun {
        options,
        revealed_emails: HashMap::new(),
        processed: 0,
        skipped: 0,
    };

    for record in &records {
        if options.limit.is_some_and(|max| run.processed >= max) {
            break;
        }
        process_record(ctx, record, &schools_url, &done_keys, &mut run, &mut report).await?;
    }

    // Finalize stats and counts.
    let after = ctx.fetcher.stats().await;
    let delta_requests = after.requests.saturating_sub(before.requests);
    report.rows = u64::try_from(run.processed).map_err(|_| CrawlError::Arithmetic {
        detail: "ihsa school count exceeds u64".to_string(),
    })?;
    report.requests = delta_requests;
    report.note(format!(
        "fetched {} Illinois schools from IHSA; {} already done",
        run.processed, run.skipped
    ));

    Ok(report)
}

/// Mutable state for one `collect` run: the run's options, the per-school reveal cache and the
/// tallies.
///
/// Tally counters saturate: they feed diagnostics only, so an impossible overflow floors at
/// `usize::MAX` instead of panicking or wrapping silently.
struct IhsaRun<'a> {
    options: &'a Options,
    /// PersonID → revealed address within the current school (a person can hold several titles).
    revealed_emails: HashMap<i64, Option<String>>,
    processed: usize,
    skipped: usize,
}

/// Fetch and parse the state-wide schools list; `None` when the request failed.
async fn fetch_school_records(
    ctx: &AdapterContext<'_>,
    report: &mut AdapterReport,
    schools_url: &str,
) -> CrawlResult<Option<Vec<SchoolRecord>>> {
    let outcome = match ctx.fetcher.get(schools_url, &ctx.fetch_options()).await {
        Ok(o) => o,
        Err(e) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!("failed to fetch IHSA schools: {e}"));
            return Ok(None);
        }
    };

    let records = parse_schools(&outcome.text())?;
    report.requests = report.requests.saturating_add(1);
    if outcome.from_cache {
        report.from_cache = report.from_cache.saturating_add(1);
    }
    Ok(Some(records))
}

/// Process one school record: parse the school, fetch its staff, emit coaches and journal the
/// school once its rows are appended.
async fn process_record(
    ctx: &AdapterContext<'_>,
    record: &SchoolRecord,
    schools_url: &str,
    done_keys: &HashSet<String>,
    run: &mut IhsaRun<'_>,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    // Skip already-processed schools (resume support).
    let journal_key = format!("IL:{}", record.school_id);
    if done_keys.contains(&journal_key) {
        run.skipped = run.skipped.saturating_add(1);
        return Ok(());
    }
    run.revealed_emails.clear();

    // Parse school.
    let Some((school, school_id)) = parse_school(record, schools_url, &run.options.observed_on)
    else {
        return Ok(());
    };
    ctx.store.append(Table::Schools, &school)?;

    let staff_url = format!("{IHSA_API}/v1/schools/{}/staff2", record.school_id);
    let Some(staff) = fetch_staff(ctx, &staff_url, record, &journal_key, run, report).await? else {
        return Ok(());
    };

    let coaches = emit_coaches(ctx, record, &staff, &school_id, &staff_url, run, report).await;

    // Append this school's coach rows, then journal it: the append commits at `SyncData` and the
    // journal write commits separately, so a journaled school never claims rows the store does not
    // hold.
    ctx.store.append_many(Table::Coaches, &coaches)?;
    journal_school(
        ctx,
        &journal_key,
        &serde_json::json!({
            "school_id": record.school_id,
            "name": record.name_formal,
        }),
    )?;

    run.processed = run.processed.saturating_add(1);
    Ok(())
}

/// Emit one coach per staff person, revealing the addresses the census pays for.
///
/// `staff2` carries a `HasEmail` flag but never the address itself; the address is behind
/// `GET /v1/schools/{id}/staff/{PersonID}/email` (the "Show email" button). One reveal per person,
/// and only for the roles the census ships: the recruiting projection reads TF/XC head coaches and
/// ADs, so a bowling coach's address is not worth a request.
async fn emit_coaches(
    ctx: &AdapterContext<'_>,
    record: &SchoolRecord,
    staff: &[StaffPerson],
    school_id: &SchoolId,
    staff_url: &str,
    run: &mut IhsaRun<'_>,
    report: &mut AdapterReport,
) -> Vec<CanonicalCoach> {
    let mut coaches = Vec::new();
    for person in staff {
        let Some(mut coach) = parse_coach(person, school_id, staff_url, &run.options.observed_on)
        else {
            continue;
        };
        if person.has_email == Some(true) && reveal_address_for(&coach) {
            let email_url = format!(
                "{IHSA_API}/v1/schools/{}/staff/{}/email",
                record.school_id, person.person_id
            );
            let revealed = match run.revealed_emails.entry(person.person_id) {
                std::collections::hash_map::Entry::Occupied(slot) => slot.get().clone(),
                std::collections::hash_map::Entry::Vacant(slot) => {
                    let value = match ctx.fetcher.get(&email_url, &ctx.fetch_options()).await {
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
                    &run.options.observed_on,
                ));
            }
        }
        if coach.professional_email.is_some() {
            report.with_email = report.with_email.saturating_add(1);
        }
        coaches.push(coach);
    }
    coaches
}

/// Journal one school's progress on `ihsa_schools`.
fn journal_school(
    ctx: &AdapterContext<'_>,
    journal_key: &str,
    details: &serde_json::Value,
) -> CrawlResult<()> {
    Ok(ctx
        .store
        .journal_done("ihsa_schools", journal_key, details)?)
}

/// Fetch and parse one school's staff list; `None` when either step failed and was journalled.
async fn fetch_staff(
    ctx: &AdapterContext<'_>,
    staff_url: &str,
    record: &SchoolRecord,
    journal_key: &str,
    run: &mut IhsaRun<'_>,
    report: &mut AdapterReport,
) -> CrawlResult<Option<Vec<StaffPerson>>> {
    let staff_outcome = match ctx.fetcher.get(staff_url, &ctx.fetch_options()).await {
        Ok(o) => o,
        Err(e) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!(
                "failed to fetch staff for school {}: {e}",
                record.school_id
            ));
            journal_school(
                ctx,
                journal_key,
                &serde_json::json!({
                    "school_id": record.school_id,
                    "name": record.name_formal,
                    "error": e.to_string(),
                }),
            )?;
            run.processed = run.processed.saturating_add(1);
            return Ok(None);
        }
    };
    report.requests = report.requests.saturating_add(1);
    if staff_outcome.from_cache {
        report.from_cache = report.from_cache.saturating_add(1);
    }

    let staff = match parse_staff(&staff_outcome.text()) {
        Ok(env) => env,
        Err(e) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!(
                "failed to parse staff for school {}: {e}",
                record.school_id
            ));
            journal_school(
                ctx,
                journal_key,
                &serde_json::json!({
                    "school_id": record.school_id,
                    "name": record.name_formal,
                    "parse_error": e.to_string(),
                }),
            )?;
            run.processed = run.processed.saturating_add(1);
            return Ok(None);
        }
    };
    Ok(Some(staff))
}
