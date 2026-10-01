mod collect;
mod map;
mod parse;
mod probe;
mod probe_utils;
mod row;
mod staff;
mod survey;
#[cfg(test)]
mod survey_tests;

pub use collect::collect;
pub use map::{
    absorb_summary, coach_entities, directory_school, CoachCounters, CoachEmission,
    DirectoryAdmission, EmissionScope,
};
pub use parse::{
    parse_directory, parse_summary, DirectoryPage, DirectorySchool, SchoolSummary, StaffMember,
    SummaryAddress, SummaryTel, TeamEntry,
};
pub use probe::{probe_one, report_json, sort_records, survey, table_line, ProbeRecord};
pub use survey::{parse_state_filter, selected_associations, ASSOCIATIONS, VERIFIED};

use census_domain::UsJurisdiction;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

pub const SOURCE_ID: &str = "coach_directories";
pub const API_HOST: &str = "https://maxinfosite-api-live.dragonflyathletics.com";

pub(crate) const REGISTERED: [(UsJurisdiction, &str); 15] = [
    (UsJurisdiction::Alabama, "AHSAA"),
    (UsJurisdiction::Arkansas, "ArkAA"),
    (UsJurisdiction::Delaware, "DIAA"),
    (UsJurisdiction::DistrictOfColumbia, "DCSAA"),
    (UsJurisdiction::Georgia, "GHSA"),
    (UsJurisdiction::Idaho, "IdHSAA"),
    (UsJurisdiction::Maryland, "MPSSAA"),
    (UsJurisdiction::Mississippi, "MHSAA"),
    (UsJurisdiction::Montana, "MHSA"),
    (UsJurisdiction::NorthCarolina, "NCHSAA"),
    (UsJurisdiction::NorthDakota, "NDHSAA"),
    (UsJurisdiction::NewMexico, "NMAA"),
    (UsJurisdiction::SouthCarolina, "SCHSL"),
    (UsJurisdiction::Tennessee, "TSSAA"),
    (UsJurisdiction::Wyoming, "WHSAA"),
];

pub(crate) const MAX_DIRECTORY_PAGES: usize = 64;

pub fn ruleset(state: UsJurisdiction) -> Option<&'static str> {
    REGISTERED
        .iter()
        .find(|(registered, _)| *registered == state)
        .map(|(_, association)| *association)
}

pub fn directory_page_url(association: &str, page: usize) -> String {
    format!("{API_HOST}/states/{association}/directory/{page}")
}

pub fn summary_url(short_code: &str) -> String {
    format!("{API_HOST}/schools/{short_code}/summary")
}

pub(crate) fn classification(levels: &BTreeMap<String, serde_json::Value>) -> Option<String> {
    levels.iter().find_map(|(key, value)| {
        let lowered = key.to_ascii_lowercase();
        let names_a_class = lowered.ends_with("class")
            || lowered.ends_with("classification")
            || lowered.ends_with("classifications");
        if !names_a_class {
            return None;
        }
        let text = value.as_str()?.trim();
        if text.is_empty() {
            None
        } else {
            Some(text.to_string())
        }
    })
}

pub(crate) fn nonempty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "tests/admission_regressions.rs"]
mod admission_regressions;
