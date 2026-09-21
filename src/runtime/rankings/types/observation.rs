use super::relay::VerifiedRelayMember;
use crate::domain::identity::AthleteId;
use crate::domain::name::CanonicalName;
use serde::{Deserialize, Serialize};

/// An individual GradeID=11 athlete candidate with row provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndividualCandidate {
    #[serde(rename = "AthleteID")]
    pub athlete_id: AthleteId,
    #[serde(rename = "AthleteName")]
    pub name: CanonicalName,
    #[serde(rename = "IDResult")]
    pub id_result: u64,
    #[serde(rename = "GradeID")]
    pub grade_id: u64,
    #[serde(rename = "TeamID")]
    pub team_id: Option<u64>,
    #[serde(rename = "TeamName")]
    pub team_name: Option<String>,
    #[serde(rename = "State")]
    pub state: Option<String>,
    #[serde(rename = "Country")]
    pub country: Option<String>,
    /// JSON pointer to the source row: groupedRankings/{group}/{row}
    /// Relative index in the corresponding observation candidate vector.
    pub record_index: u64,
    pub source_locator: Option<String>,
}

/// Returns source-located eligible candidate records plus counts.
/// Bounded observation from a parsed source page.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PageObservation {
    pub row_count: u64,
    pub id_results: Vec<u64>,
    pub grade_11_candidates: u64,
    pub total_relay_rows: u64,
    pub rows_with_roster: u64,
    pub rows_missing_roster: u64,
    pub total_relay_teams: u64,
    pub division_id: Option<u64>,
    pub season_id: Option<u64>,
    pub request_gender: Option<String>,
    pub request_page: Option<u32>,
    pub event_short: Option<String>,
    pub settings_page_depth: u64,
    /// Individual GradeID=11 athlete candidates with provenance.
    pub grade_11_candidates_list: Vec<IndividualCandidate>,
    pub verified_relay_members: Vec<VerifiedRelayMember>,
    /// Source-located ranking rows with provenance.
    pub source_rows: Vec<RankingRowObservation>,
    /// Lower bound of rows observed across all pages.
    pub min_count: u64,
    /// Count of individual identities that could not be resolved.
    pub unresolved_individual_identities: u64,
    /// Count of relay member identities that could not be resolved.
    pub unresolved_member_identities: u64,
}

impl PageObservation {
    pub fn is_valid(&self, div: u64, season: u64, page: u32) -> bool {
        self.division_id == Some(div)
            && self.season_id == Some(season)
            && self.request_page == Some(page)
    }
}

/// Typed expected page context for parser validation.
/// Native collection always Some(11) individual/None relay.
/// Historical diagnostic individual all-grade capture may explicitly expect None.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedPageContext<'a> {
    pub division_id: u64,
    pub season_id: u64,
    pub gender: &'a str,
    pub event_short: &'a str,
    pub event_id: Option<u64>,
    pub is_relay: bool,
    pub requested_grade: Option<u8>,
    pub page: u32,
}

/// One ranking row observation with provenance.
/// Row positions from actual rowNum, never per-page enumeration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankingRowObservation {
    pub result_id: u64,
    pub row_number: u64,
    pub roster_present: Option<bool>,
}
