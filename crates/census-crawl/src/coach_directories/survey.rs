mod records;
mod sampling;
mod stats;

pub use records::{
    failed, parse_state_filter, selected_associations, sort_records, state_key, table_line,
    ProbeRecord,
};
use sampling::sample_positions;
pub use sampling::{process_sampled_school, sample_rows};
pub use stats::round_half_even;

use crate::coach_directories::directory_page_url;
use crate::coach_directories::parse::{parse_directory, DirectorySchool};
use crate::net::{FetchOptions, Fetcher};
use crate::{CrawlError, CrawlResult};
use census_domain::UsJurisdiction;
use indexmap::IndexMap;
use serde::Serialize;

mod associations;
pub use associations::{ASSOCIATIONS, VERIFIED};

fn classify_error(error: &CrawlError) -> (&'static str, String) {
    match error {
        CrawlError::Fetch(source) => classify_fetch(source),
        _ => ("invariant", error.to_string()),
    }
}

fn classify_fetch(error: &crate::net::FetchError) -> (&'static str, String) {
    match error {
        crate::net::FetchError::Http { .. } => ("http", error.to_string()),
        crate::net::FetchError::RateLimited { .. } => ("rate_limited", error.to_string()),
        crate::net::FetchError::TooLarge { .. } => ("http", error.to_string()),
        crate::net::FetchError::Transport { .. } => ("transport", error.to_string()),
        crate::net::FetchError::Cache { .. } => ("invariant", error.to_string()),
        crate::net::FetchError::Timeout { .. } => ("timeout", error.to_string()),
        crate::net::FetchError::BrowserLane { .. } => ("http", error.to_string()),
        crate::net::FetchError::InvalidUrl { .. } => ("invariant", error.to_string()),
        crate::net::FetchError::Decode { .. } => ("json", error.to_string()),
        crate::net::FetchError::Encode { .. } => ("invariant", error.to_string()),
        crate::net::FetchError::Client { .. } => ("transport", error.to_string()),
        crate::net::FetchError::Invariant { .. } => ("invariant", error.to_string()),
        crate::net::FetchError::Policy { .. } => ("policy", error.to_string()),
        crate::net::FetchError::Offline { .. } => ("offline", error.to_string()),
    }
}

pub async fn probe_one(fetcher: &Fetcher, state: UsJurisdiction, ruleset: &str) -> ProbeRecord {
    let url = directory_page_url(ruleset, 1);
    let options = FetchOptions::default();
    let outcome = match fetcher.get(&url, &options).await {
        Ok(outcome) => outcome,
        Err(error) => {
            let (status, message) = classify_fetch(&error);
            return failed(state, ruleset, status, &message);
        }
    };
    let page = match parse_directory(&outcome.body) {
        Ok(page) => page,
        Err(error) => return failed(state, ruleset, "json", &error.to_string()),
    };
    let mut record = ProbeRecord::new(state_key(state), ruleset.to_string());
    record.schools = Some(page.results.len());
    record.with_address = Some(count_with_address(&page.results));
    record.pages = Some(page.total_pages);
    record.directory_total = Some(page.total_results);
    let sampled = select_sample(&page);
    record.sampled = Some(sampled.len());
    let results = process_sampled(fetcher, &sampled).await;
    let (staff, coaches, sports) = match results {
        Ok(data) => data,
        Err(error) => {
            let (status, message) = classify_error(&error);
            return failed(state, ruleset, status, &message);
        }
    };
    record.staff = Some(staff);
    record.coaches = Some(coaches);
    record.sports = Some(sports);
    let sampled_count = sampled.len().max(1);
    record.staff_per_school = Some(round_half_even(staff, sampled_count, 10));
    record.coaches_per_school = Some(round_half_even(coaches, sampled_count, 10));
    record
}

fn count_with_address(results: &[DirectorySchool]) -> usize {
    results
        .iter()
        .filter(|row| {
            row.address
                .as_deref()
                .is_some_and(|value| !value.is_empty())
        })
        .count()
}

fn select_sample(page: &crate::coach_directories::parse::DirectoryPage) -> Vec<&DirectorySchool> {
    let classified: Vec<&DirectorySchool> = page
        .results
        .iter()
        .filter(|row| !row.competition_levels.is_empty())
        .collect();
    if classified.is_empty() {
        return sample_rows(&page.results);
    }
    sample_positions(classified.len())
        .into_iter()
        .filter_map(|position| classified.get(position).copied())
        .collect()
}

async fn process_sampled(
    fetcher: &Fetcher,
    rows: &[&DirectorySchool],
) -> CrawlResult<(usize, usize, IndexMap<String, usize>)> {
    let mut staff = 0usize;
    let mut coaches = 0usize;
    let mut sports: IndexMap<String, usize> = IndexMap::new();
    for row in rows {
        let result = process_sampled_school(fetcher, row).await?;
        staff = staff.saturating_add(result.staff);
        coaches = coaches.saturating_add(result.coaches);
        for (key, count) in result.sports {
            let slot = sports.entry(key).or_insert(0);
            *slot = slot.saturating_add(count);
        }
    }
    Ok((staff, coaches, sports))
}

pub async fn survey(
    fetcher: &Fetcher,
    associations: &[(UsJurisdiction, &str)],
) -> Vec<ProbeRecord> {
    let mut records = Vec::new();
    for (state, ruleset) in associations {
        let record = probe_one(fetcher, *state, ruleset).await;
        records.push(record);
    }
    records
}

pub fn report_json(records: &[ProbeRecord]) -> CrawlResult<String> {
    let mut sorted: Vec<ProbeRecord> = records.to_vec();
    sort_records(&mut sorted);
    let mut buffer: Vec<u8> = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b" ");
    let mut serializer = serde_json::Serializer::with_formatter(&mut buffer, formatter);
    sorted
        .serialize(&mut serializer)
        .map_err(|source| CrawlError::Encode {
            table: "coach_directories".to_string(),
            source,
        })?;
    String::from_utf8(buffer).map_err(|error| CrawlError::Invariant {
        detail: format!("probe report is not utf-8: {error}"),
    })
}
