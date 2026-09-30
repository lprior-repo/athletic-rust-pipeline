use census_domain::UsJurisdiction;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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
    pub fn new(state: String, ruleset: String) -> Self {
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

pub fn failed(state: UsJurisdiction, ruleset: &str, status: &str, message: &str) -> ProbeRecord {
    ProbeRecord::new(state_key(state), ruleset.to_string()).with_error(status, message)
}

fn truncate(input: &str, max_chars: usize) -> String {
    input.chars().take(max_chars).collect()
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
    super::ASSOCIATIONS
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

pub fn sort_records(records: &mut [ProbeRecord]) {
    records.sort_by(|a, b| a.state.cmp(&b.state));
}
