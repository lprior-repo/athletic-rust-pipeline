//! The KSHSAA directory pass: fetch the directory, then append and journal each member school.
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{CanonicalCoach, SourceNamespace};
use census_domain::UsJurisdiction;
use census_store::Table;
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
    /// Restrict to these jurisdictions when the provider spans several states.
    pub states: Vec<UsJurisdiction>,
    /// School names to resolve when the provider has no bulk index.
    pub school_names: Vec<String>,
}

/// Collect this provider's schools and AD contacts into the canonical store.
///
/// Strategy:
/// 1. Fetch `https://kshsaa-api.kshsaa.org/directory/search/name/a/` — the single request returns
///    all ~526 Kansas member schools.
/// 2. Parse the JSON array; each record becomes one canonical school and one AD coach.
/// 3. Append each school and its AD coach, then journal that school, so a re-run resumes past
///    every school whose rows are already in the store.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("ks", "schools");
    report.unit = "schools".to_string();

    let before = ctx.fetcher.stats().await;

    let url = format!("{KSHSAA_API}a/");

    let outcome = match ctx.fetcher.get(&url, &ctx.fetch_options()).await {
        Ok(o) => o,
        Err(e) => {
            report.errors = report.errors.saturating_add(1);
            report.note(format!("failed to fetch KSHSAA directory: {e}"));
            return Ok(report);
        }
    };

    let records = parse_records(&outcome.text())?;
    report.requests = report.requests.saturating_add(1);
    if outcome.from_cache {
        report.from_cache = report.from_cache.saturating_add(1);
    }

    let done_keys: HashSet<String> = ctx.store.journal_keys("kshsaa_schools")?;

    let tally = collect_records(&records, ctx, options, &url, &done_keys, &mut report)?;

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

/// What one directory record contributed to the pass.
enum KsRead {
    /// The record carried no canonical school; nothing was appended and nothing journaled.
    Unreadable,
    /// The record was appended and journaled as done; `None` means it published no AD coach.
    ///
    /// Boxed: a `CanonicalCoach` is 224 bytes, so an unboxed payload would make every value of
    /// this enum - including the `Unreadable` ones the walk drops - carry that size.
    Read(Box<Option<CanonicalCoach>>),
}

/// What one directory pass counted; the email count is recorded on the report as each record is
/// read.
struct KsTally {
    processed: usize,
    skipped: usize,
    skipped_no_ad: usize,
}

/// Walk the directory once: journalled schools are counted as skipped, the rest are appended and
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
    };

    for record in records {
        if let Some(max) = limit {
            if tally.processed >= max {
                break;
            }
        }

        let journal_key = format!("KS:{}", record.identifier);
        if done_keys.contains(&journal_key) {
            tally.skipped = tally.skipped.saturating_add(1);
            continue;
        }

        match collect_record(record, ctx, url, &options.observed_on, &journal_key)? {
            KsRead::Unreadable => continue,
            KsRead::Read(coach) => match coach.as_ref() {
                Some(coach) => {
                    if coach.professional_email.is_some() || coach.personal_email.is_some() {
                        report.with_email = report.with_email.saturating_add(1);
                    }
                }
                None => tally.skipped_no_ad = tally.skipped_no_ad.saturating_add(1),
            },
        }
        tally.processed = tally.processed.saturating_add(1);
    }
    Ok(tally)
}

/// One directory record: append its rows, then journal the school as done.
///
/// The append commits at `SyncData` and the journal write commits separately, so the journal entry
/// goes last: a journaled school never claims rows the store does not hold.
fn collect_record(
    record: &KshsaaRecord,
    ctx: &AdapterContext<'_>,
    url: &str,
    observed_on: &str,
    journal_key: &str,
) -> CrawlResult<KsRead> {
    let Some((school, school_id)) = parse_school(record, url, observed_on) else {
        return Ok(KsRead::Unreadable);
    };

    let coach = parse_ad_coach(record, &school_id, url, observed_on);

    let mut batch = ctx.store.write_batch();
    batch.append_many(Table::Schools, std::slice::from_ref(&school))?;
    ctx.observe_school(
        &SourceNamespace::association_school(super::ASSOCIATION),
        &school,
    )?;
    if let Some(row) = coach.as_ref() {
        batch.append_many(Table::Coaches, std::slice::from_ref(row))?;
    }
    batch.journal_done(
        "kshsaa_schools",
        journal_key,
        &serde_json::json!({
            "identifier": record.identifier,
            "school_name": record.school_name,
        }),
    )?;
    batch.commit()?;
    Ok(KsRead::Read(Box::new(coach)))
}
