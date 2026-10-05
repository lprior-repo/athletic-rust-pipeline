use super::parse::{parse_directory, parse_summary};
use super::survey::state_key;
use super::{directory_page_url, map, summary_url, SOURCE_ID};
use crate::{CrawlError, CrawlResult};
use census_domain::model::Sport;
use census_domain::UsJurisdiction;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

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
        self.error = Some(error.chars().take(200).collect());
        self
    }
}

async fn fetch_and_parse_directory(
    fetcher: &crate::net::Fetcher,
    url: &str,
) -> CrawlResult<super::parse::DirectoryPage> {
    let outcome = fetcher
        .get(url, &crate::net::FetchOptions::default())
        .await?;
    parse_directory(&outcome.body)
}

fn compute_sampling_metrics(
    page: &super::parse::DirectoryPage,
    sampled: &[&super::parse::DirectorySchool],
    staff: usize,
    coaches: usize,
) -> (usize, usize, f64, f64) {
    let schools = page.results.len();
    let with_address = page
        .results
        .iter()
        .filter(|row| row.address.as_deref().is_some_and(|v| !v.is_empty()))
        .count();
    let sampled_count = sampled.len().max(1);
    let staff_per_school = super::probe_utils::round_half_even(staff, sampled_count, 10);
    let coaches_per_school = super::probe_utils::round_half_even(coaches, sampled_count, 10);
    (schools, with_address, staff_per_school, coaches_per_school)
}

async fn probe_sampled_schools(
    fetcher: &crate::net::Fetcher,
    sampled: &[&super::parse::DirectorySchool],
) -> CrawlResult<(usize, usize, IndexMap<String, usize>)> {
    let options = crate::net::FetchOptions::default();
    let mut staff_count: usize = 0;
    let mut coach_count: usize = 0;
    let mut sports: IndexMap<String, usize> = IndexMap::new();

    for row in sampled {
        let short_code = match row.short_code.as_deref() {
            Some(code) if !code.is_empty() => code.to_string(),
            _ => continue,
        };

        let summary_url = summary_url(&short_code);
        let summary_outcome = fetcher.get(&summary_url, &options).await?;
        let summary = parse_summary(&summary_outcome.body)?;

        staff_count = staff_count.saturating_add(summary.staff.len());

        let emission = map::probe_coach_entities(
            &summary,
            &census_domain::model::SchoolId::mint("sch", &["survey", short_code.as_str()]),
            map::Capture {
                url: &summary_outcome.url,
                observed_on: &summary_outcome.fetched_at,
                sha256: &summary_outcome.content_digest,
            },
        )?;

        for entity in emission.coaches {
            coach_count = coach_count.saturating_add(1);
            let sport_key = artifact_sport_key(entity.sport);
            let counter = sports.entry(sport_key).or_insert(0);
            *counter = counter.saturating_add(1);
        }
    }

    Ok((staff_count, coach_count, sports))
}

pub async fn probe_one(
    fetcher: &crate::net::Fetcher,
    state: UsJurisdiction,
    ruleset: &str,
) -> ProbeRecord {
    let url = directory_page_url(ruleset, 1);
    let page = match fetch_and_parse_directory(fetcher, &url).await {
        Ok(page) => page,
        Err(error) => {
            let (status, message) = super::probe_utils::classify_error(&error);
            return ProbeRecord::new(state_key(state), ruleset.to_string())
                .with_error(status, &message);
        }
    };

    let classified: Vec<&super::parse::DirectorySchool> = page
        .results
        .iter()
        .filter(|row| !row.competition_levels.is_empty())
        .collect();

    let sampled = if classified.is_empty() {
        super::probe_utils::sample_rows(&page.results)
    } else {
        super::probe_utils::sample_rows(&classified)
    };

    let (staff, coaches, sports) = match probe_sampled_schools(fetcher, &sampled).await {
        Ok(result) => result,
        Err(error) => {
            let (status, message) = super::probe_utils::classify_error(&error);
            return ProbeRecord::new(state_key(state), ruleset.to_string())
                .with_error(status, &message);
        }
    };

    let (schools, with_address, staff_ps, coaches_ps) =
        compute_sampling_metrics(&page, &sampled, staff, coaches);

    ProbeRecord {
        state: state_key(state),
        ruleset: ruleset.to_string(),
        status: "ok".to_string(),
        error: None,
        schools: Some(schools),
        with_address: Some(with_address),
        pages: Some(page.total_pages),
        directory_total: Some(page.total_results),
        sampled: Some(sampled.len()),
        staff: Some(staff),
        coaches: Some(coaches),
        sports: Some(sports),
        staff_per_school: Some(staff_ps),
        coaches_per_school: Some(coaches_ps),
    }
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

fn artifact_sport_key(sport: Option<Sport>) -> String {
    match sport {
        Some(Sport::IndoorTrack) | Some(Sport::OutdoorTrack) => "Track".to_string(),
        Some(Sport::CrossCountry) => "CrossCountry".to_string(),
        None => "AthleticDirector".to_string(),
    }
}

pub fn table_line(record: &ProbeRecord) -> String {
    format!(
        "{state:<3} {ruleset:<10} {status:<16} rows={rows:>5} pages={pages} \
staff/school={staff:>5} coach/school={coaches:>5} {sports}",
        state = record.state,
        ruleset = record.ruleset,
        status = record.status,
        rows = record.schools.map_or(0, |value| value),
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

pub fn report_json(records: &[ProbeRecord]) -> CrawlResult<String> {
    let mut sorted: Vec<&ProbeRecord> = records.iter().collect();
    sorted.sort_by(|a, b| a.state.cmp(&b.state));
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
