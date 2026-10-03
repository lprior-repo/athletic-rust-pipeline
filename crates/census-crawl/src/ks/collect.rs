use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::{CanonicalCoach, SourceNamespace};
use census_domain::UsJurisdiction;
use census_store::Table;
use std::collections::HashSet;

use super::parse::{parse_ad_coach, parse_school};
use super::wire::{parse_records, KshsaaRecord};

const KSHSAA_API: &str = "https://kshsaa-api.kshsaa.org/directory/search/name/";

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

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
    report.rows = u64::try_from(tally.processed).map_or(u64::MAX, |value| value);
    report.requests = delta_requests;
    report.note(format!(
        "fetched {} schools from KSHSAA; {} already done; {} skipped (no AD name)",
        tally.processed, tally.skipped, tally.skipped_no_ad
    ));

    Ok(report)
}

enum KsRead {
    Unreadable,
    Read(Box<Option<CanonicalCoach>>),
}

struct KsTally {
    processed: usize,
    skipped: usize,
    skipped_no_ad: usize,
}

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
    batch.append_many(
        Table::SourceObservations,
        ctx.school_observation(
            &SourceNamespace::association_school(super::ASSOCIATION),
            &school,
        )
        .as_slice(),
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
