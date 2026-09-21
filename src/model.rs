use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const SOURCE_HEADERS: &[&str] = &[
    "Person First",
    "Person Last",
    "Person Email",
    "Address Mailing / Permanent Street Combined",
    "Address Mailing / Permanent City",
    "Address Mailing / Permanent Region",
    "Address Mailing / Permanent Postal",
    "Sports Created Date",
    "Sports Sport",
    "Sports Rating",
    "Origin Source Date",
    "Origin Source",
    "Schools Name",
];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkbookStats {
    pub sheets: Vec<SheetStats>,
    pub actual_data_rows: u64,
    pub selected_prospects: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SheetStats {
    pub name: String,
    pub declared_dimension: Option<String>,
    pub xml_rows: u64,
    pub actual_data_rows: u64,
    pub last_actual_row: u32,
    pub headers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceRecord {
    pub source_key: String,
    pub sheet: String,
    pub excel_row: u32,
    pub fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Prospect {
    pub source_key: String,
    pub sheet: String,
    pub excel_row: u32,
    pub first_name: String,
    pub last_name: String,
    pub school: String,
    pub city: String,
    pub state: String,
    pub sport: String,
    pub expected_graduation_year: Option<i32>,
    #[serde(default)]
    pub source_fields: BTreeMap<String, String>,
}

impl Prospect {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name.trim(), self.last_name.trim())
            .trim()
            .to_owned()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Mark {
    pub event: String,
    pub canonical_event: String,
    pub mark: String,
    pub season: String,
    pub date: String,
    pub meet_name: String,
    pub wind: Option<String>,
    pub source_url: String,
    pub is_pr_claimed: bool,
    pub parsed_value: Option<f64>,
    pub valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Candidate {
    pub profile_url: String,
    pub search_title: String,
    pub search_snippet: String,
    pub athlete_name: String,
    pub school: String,
    pub location: String,
    #[serde(default)]
    pub athlete_id: Option<u64>,
    pub graduation_year: Option<i32>,
    pub sports: Vec<String>,
    pub marks: Vec<Mark>,
    pub page_retrieved: bool,
    pub evidence_text: String,
    pub evidence_urls: Vec<String>,
    pub deterministic_score: f64,
    pub name_score: f64,
    pub school_score: f64,
    pub location_score: f64,
    #[serde(default)]
    pub sport_score: f64,
    pub corroborated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelDecision {
    pub decision: String,
    pub candidate_index: Option<usize>,
    pub confidence: f64,
    pub track_confirmed: bool,
    pub xc_confirmed: bool,
    pub reason: String,
    pub model_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MatchRecord {
    pub source_key: String,
    pub prospect: Prospect,
    pub status: String,
    /// Evidence-only first pass, retained independently of subsequent AI review.
    #[serde(default)]
    pub deterministic_decision: Option<ModelDecision>,
    #[serde(default)]
    pub hint_count: usize,
    #[serde(default)]
    pub ai_logic: String,
    pub score: f64,
    pub selected_candidate_index: Option<usize>,
    pub selected_profile_url: String,
    pub selected_name: String,
    pub selected_school: String,
    pub selected_location: String,
    pub track_confirmed: bool,
    pub xc_confirmed: bool,
    pub best_marks: BTreeMap<String, Mark>,
    pub candidates: Vec<Candidate>,
    pub model_decision: ModelDecision,
    pub notes: String,
    pub processed_at_unix: u64,
}

pub fn ordered_source_headers(records: &[MatchRecord]) -> Vec<String> {
    let known = SOURCE_HEADERS
        .iter()
        .map(|header| (*header).to_owned())
        .collect::<Vec<_>>();
    let known_set = SOURCE_HEADERS.iter().copied().collect::<BTreeSet<_>>();
    let extras = records
        .iter()
        .flat_map(|record| record.prospect.source_fields.keys())
        .filter(|header| !known_set.contains(header.as_str()))
        .cloned()
        .collect::<BTreeSet<_>>();
    known.into_iter().chain(extras).collect()
}
