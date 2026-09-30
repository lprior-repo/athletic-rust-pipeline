use super::parse::{parse_directory, parse_summary};
use super::{directory_page_url, map, summary_url, SOURCE_ID};
use crate::{CrawlError, CrawlResult};
use census_domain::model::Sport;
use census_domain::UsJurisdiction;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const ASSOCIATIONS: [(UsJurisdiction, &str); 51] = [
    (UsJurisdiction::Alaska, "ASAA"),
    (UsJurisdiction::Alabama, "AHSAA"),
    (UsJurisdiction::Arkansas, "ArkAA"),
    (UsJurisdiction::Arizona, "AIA"),
    (UsJurisdiction::California, "CIF"),
    (UsJurisdiction::Colorado, "CHSAA"),
    (UsJurisdiction::Connecticut, "CIAC"),
    (UsJurisdiction::DistrictOfColumbia, "DCSAA"),
    (UsJurisdiction::Delaware, "DIAA"),
    (UsJurisdiction::Florida, "FHSAA"),
    (UsJurisdiction::Georgia, "GHSA"),
    (UsJurisdiction::Hawaii, "HHSAA"),
    (UsJurisdiction::Iowa, "IoHSAA"),
    (UsJurisdiction::Idaho, "IdHSAA"),
    (UsJurisdiction::Illinois, "IHSA"),
    (UsJurisdiction::Indiana, "InHSAA"),
    (UsJurisdiction::Kansas, "KSHSAA"),
    (UsJurisdiction::Kentucky, "KHSAA"),
    (UsJurisdiction::Louisiana, "LHSAA"),
    (UsJurisdiction::Massachusetts, "MIAA"),
    (UsJurisdiction::Maryland, "MPSSAA"),
    (UsJurisdiction::Maine, "MPA"),
    (UsJurisdiction::Michigan, "MichHSAA"),
    (UsJurisdiction::Minnesota, "MSHSL"),
    (UsJurisdiction::Missouri, "MSHSAA"),
    (UsJurisdiction::Mississippi, "MHSAA"),
    (UsJurisdiction::Montana, "MHSA"),
    (UsJurisdiction::NorthCarolina, "NCHSAA"),
    (UsJurisdiction::NorthDakota, "NDHSAA"),
    (UsJurisdiction::Nebraska, "NSAA"),
    (UsJurisdiction::NewHampshire, "NHIAA"),
    (UsJurisdiction::NewJersey, "NJSIAA"),
    (UsJurisdiction::NewMexico, "NMAA"),
    (UsJurisdiction::Nevada, "NIAA"),
    (UsJurisdiction::NewYork, "NYSPHSAA"),
    (UsJurisdiction::Ohio, "OHSAA"),
    (UsJurisdiction::Oklahoma, "OSSAA"),
    (UsJurisdiction::Oregon, "OSAA"),
    (UsJurisdiction::Pennsylvania, "PIAA"),
    (UsJurisdiction::RhodeIsland, "RIIL"),
    (UsJurisdiction::SouthCarolina, "SCHSL"),
    (UsJurisdiction::SouthDakota, "SDHSAA"),
    (UsJurisdiction::Tennessee, "TSSAA"),
    (UsJurisdiction::Texas, "UIL"),
    (UsJurisdiction::Utah, "UHSAA"),
    (UsJurisdiction::Virginia, "VHSL"),
    (UsJurisdiction::Vermont, "VPA"),
    (UsJurisdiction::Washington, "WIAA"),
    (UsJurisdiction::Wisconsin, "WisIAA"),
    (UsJurisdiction::WestVirginia, "WVSSAC"),
    (UsJurisdiction::Wyoming, "WHSAA"),
];

pub const VERIFIED: [(UsJurisdiction, usize, usize, &str); 15] = [
    (
        UsJurisdiction::Alabama,
        793,
        1,
        "19.2 staff and 5.0 census rows per sampled school",
    ),
    (
        UsJurisdiction::Arkansas,
        521,
        1,
        "33.5 staff and 6.0 census rows per sampled school",
    ),
    (
        UsJurisdiction::DistrictOfColumbia,
        112,
        1,
        "14.5 staff and 1.5 census rows per sampled school",
    ),
    (
        UsJurisdiction::Delaware,
        320,
        1,
        "14.0 staff and 3.0 census rows per sampled school",
    ),
    (
        UsJurisdiction::Georgia,
        2825,
        3,
        "46.8 staff and 7.5 census rows per sampled school",
    ),
    (
        UsJurisdiction::Idaho,
        461,
        1,
        "21.5 staff and 6.5 census rows per sampled school",
    ),
    (
        UsJurisdiction::Maryland,
        215,
        1,
        "16.5 staff and 1.8 census rows per sampled school",
    ),
    (
        UsJurisdiction::Mississippi,
        1151,
        2,
        "24.8 staff and 4.2 census rows per sampled school",
    ),
    (
        UsJurisdiction::Montana,
        358,
        1,
        "28.8 staff and 11.2 census rows per sampled school",
    ),
    (
        UsJurisdiction::NorthCarolina,
        452,
        1,
        "53.5 staff and 20.5 census rows per sampled school",
    ),
    (
        UsJurisdiction::NorthDakota,
        548,
        1,
        "2.0 staff and 0.8 census rows per sampled school",
    ),
    (
        UsJurisdiction::NewMexico,
        751,
        1,
        "19.5 staff and 3.5 census rows per sampled school",
    ),
    (
        UsJurisdiction::SouthCarolina,
        1278,
        2,
        "24.0 staff and 3.0 census rows per sampled school",
    ),
    (
        UsJurisdiction::Tennessee,
        1675,
        2,
        "11.2 staff and 3.5 census rows per sampled school",
    ),
    (
        UsJurisdiction::Wyoming,
        93,
        1,
        "11.0 staff and 3.2 census rows per sampled school",
    ),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeRecord {
    pub state: String,
    pub ruleset: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schools: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_address: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory_total: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampled: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staff: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coaches: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sports: Option<IndexMap<String, usize>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub staff_per_school: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub coaches_per_school: Option<f64>,
}

impl ProbeRecord {
    fn new(state: String, ruleset: String) -> Self {
        Self {
            state,
            ruleset,
            status: "ok".to_string(),
            error: None,
            schools: None,
            with_address: None,
            pages: None,
            directory_total: None,
            sampled: None,
            staff: None,
            coaches: None,
            sports: None,
            staff_per_school: None,
            coaches_per_school: None,
        }
    }

    fn with_error(mut self, status: &str, error: &str) -> Self {
        self.status = status.to_string();
        self.error = Some(truncate(error, 200));
        self
    }
}

fn failed(state: UsJurisdiction, ruleset: &str, status: &str, message: &str) -> ProbeRecord {
    ProbeRecord::new(state_key(state), ruleset.to_string()).with_error(status, message)
}

fn truncate(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
}

fn classify_error(error: &crate::net::FetchError) -> (&'static str, String) {
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

pub(crate) fn round_half_even(numerator: usize, denominator: usize, scale: usize) -> f64 {
    if denominator == 0 {
        return 0.0;
    }
    let scaled = numerator.saturating_mul(scale);
    let quotient = scaled.checked_div(denominator).unwrap_or(0);
    let remainder = scaled.checked_rem(denominator).unwrap_or(0);
    let doubled = remainder.saturating_mul(2);
    let tie = doubled == denominator && quotient.checked_rem(2).unwrap_or(0) == 1;
    let adjusted = if doubled > denominator || tie {
        quotient.saturating_add(1)
    } else {
        quotient
    };
    let rounded = i32::try_from(adjusted).map_or(f64::from(i32::MAX), f64::from);
    let divisor = i32::try_from(scale).map_or(f64::from(i32::MAX), f64::from);
    rounded / divisor
}

pub fn state_key(state: UsJurisdiction) -> String {
    match state {
        UsJurisdiction::DistrictOfColumbia => "DC".to_string(),
        other => other.to_string(),
    }
}

pub fn parse_state_filter(states: &str) -> BTreeSet<String> {
    states
        .split(',')
        .map(|part| part.trim().to_ascii_uppercase())
        .filter(|part| !part.is_empty())
        .collect()
}

pub fn selected_associations(wanted: &BTreeSet<String>) -> Vec<(UsJurisdiction, &'static str)> {
    ASSOCIATIONS
        .iter()
        .copied()
        .filter(|(state, _)| wanted.is_empty() || wanted.contains(&state_key(*state)))
        .collect()
}

pub fn table_line(record: &ProbeRecord) -> String {
    format!(
        "{state:<3} {ruleset:<10} {status:<16} rows={rows:>5} pages={pages} \
staff/school={staff:>5} coach/school={coaches:>5} {sports}",
        state = record.state,
        ruleset = record.ruleset,
        status = record.status,
        rows = record.schools.unwrap_or(0),
        pages = record
            .pages
            .map_or_else(|| "?".to_string(), |value| value.to_string()),
        staff = record
            .staff_per_school
            .map_or_else(|| "0".to_string(), per_school),
        coaches = record
            .coaches_per_school
            .map_or_else(|| "0".to_string(), per_school),
        sports = sports_repr(record.sports.as_ref()),
    )
}

fn per_school(value: f64) -> String {
    format!("{value:.1}")
}

fn sports_repr(sports: Option<&IndexMap<String, usize>>) -> String {
    let Some(sports) = sports else {
        return "{}".to_string();
    };
    let mut rendered = String::from("{");
    for (position, (key, count)) in sports.iter().enumerate() {
        if position > 0 {
            rendered.push_str(", ");
        }
        rendered.push('\'');
        rendered.push_str(key);
        rendered.push_str("': ");
        rendered.push_str(count.to_string().as_str());
    }
    rendered.push('}');
    rendered
}

pub async fn probe_one(
    fetcher: &crate::net::Fetcher,
    state: UsJurisdiction,
    ruleset: &str,
) -> ProbeRecord {
    let url = directory_page_url(ruleset, 1);
    let options = crate::net::FetchOptions::default();
    let outcome = match fetcher.get(&url, &options).await {
        Ok(outcome) => outcome,
        Err(error) => {
            let (status, message) = classify_error(&error);
            return failed(state, ruleset, status, &message);
        }
    };

    let page = match parse_directory(&outcome.body) {
        Ok(page) => page,
        Err(error) => return failed(state, ruleset, "json", &error.to_string()),
    };

    let mut record = ProbeRecord::new(state_key(state), ruleset.to_string());

    let schools = page.results.len();
    let with_address = page
        .results
        .iter()
        .filter(|row| {
            row.address
                .as_deref()
                .is_some_and(|value| !value.is_empty())
        })
        .count();

    record.schools = Some(schools);
    record.with_address = Some(with_address);
    record.pages = Some(page.total_pages);
    record.directory_total = Some(page.total_results);

    let classified: Vec<&super::parse::DirectorySchool> = page
        .results
        .iter()
        .filter(|row| !row.competition_levels.is_empty())
        .collect();

    let sampled = if classified.is_empty() {
        sample_rows(&page.results)
    } else {
        sample_rows(&classified)
    };

    record.sampled = Some(sampled.len());

    let mut staff_count: usize = 0;
    let mut coach_count: usize = 0;
    let mut sports: IndexMap<String, usize> = IndexMap::new();

    for row in &sampled {
        let short_code = match row.short_code.as_deref() {
            Some(code) if !code.is_empty() => code.to_string(),
            _ => continue,
        };

        let summary_url = summary_url(&short_code);
        let summary_outcome = match fetcher.get(&summary_url, &options).await {
            Ok(outcome) => outcome,
            Err(error) => {
                let (status, message) = classify_error(&error);
                return failed(state, ruleset, status, &message);
            }
        };

        let summary = match parse_summary(&summary_outcome.body) {
            Ok(summary) => summary,
            Err(error) => return failed(state, ruleset, "json", &error.to_string()),
        };

        staff_count = staff_count.saturating_add(summary.staff.len());

        let emission = match map::coach_entities(
            &summary,
            &census_domain::model::SchoolId::mint("sch", &["survey", short_code.as_str()]),
            &summary_url,
            "2026-09-29",
            map::EmissionScope::Probe,
        ) {
            Ok(emission) => emission,
            Err(error) => return failed(state, ruleset, "map", &error.to_string()),
        };

        for entity in emission.coaches {
            coach_count = coach_count.saturating_add(1);
            let sport_key = artifact_sport_key(entity.sport);
            let counter = sports.entry(sport_key).or_insert(0);
            *counter = counter.saturating_add(1);
        }
    }

    record.staff = Some(staff_count);
    record.coaches = Some(coach_count);
    record.sports = Some(sports);

    let sampled_count = sampled.len().max(1);
    record.staff_per_school = Some(round_half_even(staff_count, sampled_count, 10));
    record.coaches_per_school = Some(round_half_even(coach_count, sampled_count, 10));

    record
}

pub(crate) fn sample_rows<T: std::borrow::Borrow<super::parse::DirectorySchool>>(
    rows: &[T],
) -> Vec<&super::parse::DirectorySchool> {
    let step = rows.len().saturating_div(4).max(1);
    rows.iter()
        .step_by(step)
        .take(4)
        .map(std::borrow::Borrow::borrow)
        .collect()
}

pub async fn survey(
    fetcher: &crate::net::Fetcher,
    associations: &[(UsJurisdiction, &str)],
) -> Vec<ProbeRecord> {
    let mut records = Vec::new();
    for (state, ruleset) in associations {
        let record = probe_one(fetcher, *state, ruleset).await;
        records.push(record);
    }
    records
}

fn artifact_sport_key(sport: Option<Sport>) -> String {
    match sport {
        Some(Sport::IndoorTrack) | Some(Sport::OutdoorTrack) => "Track".to_string(),
        Some(Sport::CrossCountry) => "CrossCountry".to_string(),
        None => "AthleticDirector".to_string(),
    }
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
            table: SOURCE_ID.to_string(),
            source,
        })?;
    String::from_utf8(buffer).map_err(|error| CrawlError::Invariant {
        detail: format!("probe report is not utf-8: {error}"),
    })
}

pub fn sort_records(records: &mut [ProbeRecord]) {
    records.sort_by(|a, b| a.state.cmp(&b.state));
}
