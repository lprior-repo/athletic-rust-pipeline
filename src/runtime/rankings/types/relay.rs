use crate::domain::identity::AthleteId;
use crate::domain::name::CanonicalName;
use serde::{Deserialize, Serialize};

/// Relay roster member from the application UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayMember {
    #[serde(rename = "SortID")]
    pub sort_id: u64,
    #[serde(rename = "IDAthlete")]
    pub athlete_id: u64,
    #[serde(rename = "AthleteName")]
    pub name: String,
    pub handle: String,
    #[serde(rename = "PhotoUrl", default)]
    pub photo_url: Option<String>,
    #[serde(rename = "GradeID")]
    pub grade_id: u64,
}

/// Relay team from the application UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayTeam {
    #[serde(rename = "IDResult")]
    pub id_result: u64,
    #[serde(rename = "RelayTeamID")]
    pub team_id: u64,
    pub members: Vec<RelayMember>,
}

/// A relay ranking row with the join keys for roster matching.
/// All relay rows have GradeID=99 (system placeholder).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayRow {
    #[serde(rename = "IDResult")]
    pub id_result: u64,
    #[serde(rename = "AthleteID")]
    pub athlete_id: u64,
    #[serde(rename = "TeamID")]
    pub team_id: u64,
    #[serde(rename = "AthleteName")]
    pub athlete_name: String,
    #[serde(rename = "GradeID")]
    pub grade_id: u64,
}

/// A verified relay member: member data + row provenance.
/// Only emitted when roster.RelayTeamID == row.AthleteID and member.GradeID=11.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedRelayMember {
    #[serde(rename = "IDAthlete")]
    pub athlete_id: AthleteId,
    #[serde(rename = "AthleteName")]
    pub name: CanonicalName,
    #[serde(rename = "Handle")]
    pub handle: String,
    #[serde(rename = "PhotoUrl", default)]
    pub photo_url: Option<String>,
    #[serde(rename = "GradeID")]
    pub grade_id: u64,
    #[serde(rename = "IDResult")]
    pub id_result: u64,
    pub roster_relay_team_id: u64,
    pub row_athlete_id: u64,
    /// JSON pointer to source row: groupedRankings/{group}/{row}
    pub row_locator: Option<String>,
    /// JSON pointer to source member: relayTeams/{IDResult}/Members/{index}
    pub member_locator: Option<usize>,
}

/// Relay roster indexed by IDResult string key.
pub type RelayRoster = std::collections::HashMap<String, RelayTeam>;
