//! The KSHSAA directory pass: fetch the directory, journal it and append what it carries.
use crate::sources::{AdapterContext, AdapterReport, CrawlResult};
use crate::store::Table;
use census_domain::model::{CanonicalCoach, CanonicalSchool};
use std::collections::HashSet;

use super::parse::{parse_ad_coach, parse_school};
use super::wire::{parse_records, KshsaaRecord};

/// API base URL for the name-search directory endpoint.
const KSHSAA_API: &str = "https://kshsaa-api.kshsaa.org/directory/search/name/";

/// Adapter options.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Stop after this many schools (smoke runs).
    pub limit: Option<usize>,
    /// Ignore cached HTTP bodies and re-fetch.
    pub refresh: bool,
    /// ISO date stamped into evidence.
    pub observed_on: String,
    /// Restrict to these state codes when the provider spans several states.
    pub states: Vec<String>,
    /// School names to resolve when the provider has no bulk index.
    pub school_names: Vec<String>,
}

/// Collect this provider's schools and AD contacts into the canonical store.
///
/// Strategy:
/// 1. Fetch `https://kshsaa-api.kshsaa.org/directory/search/name/a/` — the single request returns
///    all ~526 Kansas member schools.
/// 2. Parse the JSON array; each record becomes one canonical school and one AD coach.
/// 3. Journal progress per school so a re-run resumes without re-processing.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("ks", "schools");
    report.unit = "schools".to_string();

    // Snapshot fetcher stats before work.
    let before = ctx.fetcher.stats().await;

    // Build the API URL. The `a` endpoint returns the full directory.
    let url = format!("{KSHSAA_API}a/");

    // Fetch with caching (respect `options.refresh`).
    let outcome = match ctx.fetcher.get(&url, &ctx.fetch_options()).await {
        Ok(o) => o,
        Err(e) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!("failed to fetch KSHSAA directory: {e}"));
            return Ok(report);
        }
    };

    // Parse JSON.
    let records = parse_records(&outcome.text())?;
    report.requests = report.requests.saturating_add(1);
    if outcome.from_cache {
        report.from_cache = report.from_cache.saturating_add(1);
    }

    // Load the resume set (keys already journalled in this phase).
    let done_keys: HashSet<String> = ctx.store.journal_keys("kshsaa_schools")?;

    let tally = collect_records(&records, ctx, options, &url, &done_keys, &mut report)?;
    append_ks_entities(ctx, &tally.schools, &tally.coaches)?;

    // Finalize stats and counts.
    let after = ctx.fetcher.stats().await;
    let delta_requests = after.requests.saturating_sub(before.requests);
    report.rows = u64::try_from(tally.processed).unwrap_or(u64::MAX);
    report.requests = delta_requests;
    report.note(format!(
        "fetched {} schools from KSHSAA; {} already done; {} skipped (no AD name)",
        tally.processed, tally.skipped, tally.skipped_no_ad
    ));

    Ok(report)
}

/// What one journaled directory record contributed.
struct KsRecord {
    school: CanonicalSchool,
    coach: Option<CanonicalCoach>,
}

/// What one directory pass accumulated, besides the email count, which is recorded on the report as
/// each record is read.
struct KsTally {
    processed: usize,
    skipped: usize,
    skipped_no_ad: usize,
    schools: Vec<CanonicalSchool>,
    coaches: Vec<CanonicalCoach>,
}

/// Walk the directory once: journalled schools are counted as skipped, the rest are read and
/// journalled as done so a re-run resumes past them.
fn collect_records(
    records: &[KshsaaRecord],
    ctx: &AdapterContext<'_>,
    options: &Options,
    url: &str,
    done_keys: &HashSet<String>,
    report: &mut AdapterReport,
) -> CrawlResult<KsTally> {
    let limit = options.limit;
    let mut tally = KsTally {
        processed: 0,
        skipped: 0,
        skipped_no_ad: 0,
        schools: Vec::new(),
        coaches: Vec::new(),
    };

    for record in records {
        // Honour limit.
        if let Some(max) = limit {
            if tally.processed >= max {
                break;
            }
        }

        // Skip already-processed schools (resume support).
        let journal_key = format!("KS:{}", record.identifier);
        if done_keys.contains(&journal_key) {
            tally.skipped = tally.skipped.saturating_add(1);
            continue;
        }

        let Some(read) = collect_record(record, ctx, url, &options.observed_on, &journal_key)?
        else {
            continue;
        };
        if let Some(coach) = read.coach {
            if coach.professional_email.is_some() {
                report.with_email = report.with_email.saturating_add(1);
            }
            tally.coaches.push(coach);
        } else {
            tally.skipped_no_ad = tally.skipped_no_ad.saturating_add(1);
        }
        tally.schools.push(read.school);
        tally.processed = tally.processed.saturating_add(1);
    }
    Ok(tally)
}

/// One directory record: its school, the AD coach it publishes (absent when the AD name is empty),
/// and the journal entry that marks the school done.
fn collect_record(
    record: &KshsaaRecord,
    ctx: &AdapterContext<'_>,
    url: &str,
    observed_on: &str,
    journal_key: &str,
) -> CrawlResult<Option<KsRecord>> {
    // Parse school.
    let Some((school, school_id)) = parse_school(record, url, observed_on) else {
        return Ok(None);
    };

    // Parse AD coach.
    let coach = parse_ad_coach(record, &school_id, url, observed_on);

    // Journal this school as done.
    ctx.store.journal_done(
        "kshsaa_schools",
        journal_key,
        &serde_json::json!({
            "identifier": record.identifier,
            "school_name": record.school_name,
        }),
    )?;
    Ok(Some(KsRecord { school, coach }))
}

/// Append all schools and coaches of one pass to the store.
fn append_ks_entities(
    ctx: &AdapterContext<'_>,
    schools: &[CanonicalSchool],
    coaches: &[CanonicalCoach],
) -> CrawlResult<()> {
    if !schools.is_empty() {
        ctx.store.append_many(Table::Schools, schools)?;
    }
    if !coaches.is_empty() {
        ctx.store.append_many(Table::Coaches, coaches)?;
    }
    Ok(())
}
