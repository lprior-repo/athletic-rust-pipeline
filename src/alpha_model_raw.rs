use serde::{Deserialize, Serialize};

/// One row inside `groupedRankings` — supports both nested Results and
/// confirmed flattened row shapes.
///
/// Unknown fields are silently ignored.
///
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RawRankingRecord {
    #[serde(rename = "AthleteID")]
    pub athlete_id: u64,
    #[serde(rename = "AthleteName")]
    pub athlete_name: String,
    #[serde(rename = "GradeID")]
    pub grade_id: u64,
    #[serde(rename = "TeamName")]
    pub team_name: String,
    #[serde(rename = "State")]
    pub state: String,
    /// Nested results array. `None` when the API omits the field.
    #[serde(rename = "Results")]
    pub results: Option<Vec<RawRankingResult>>,
    // Flattened row fields — all optional, validated downstream.
    #[serde(rename = "IDResult")]
    pub id_result: Option<u64>,
    #[serde(rename = "EventShort")]
    pub event_short: Option<String>,
    #[serde(rename = "Measure")]
    pub measure: Option<String>,
    #[serde(rename = "ResultDate")]
    pub result_date: Option<String>,
    #[serde(rename = "SeasonID")]
    pub season_id: Option<i32>,
    #[serde(rename = "Wind")]
    pub wind: Option<Option<String>>,
    #[serde(rename = "MeetID")]
    pub meet_id: Option<u64>,
    #[serde(rename = "MeetName")]
    pub meet_name: Option<String>,
}

/// One result entry inside a `RawRankingRecord`.
///
/// All fields are required except Wind (optional).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RawRankingResult {
    #[serde(rename = "MeetID")]
    pub meet_id: u64,
    #[serde(rename = "MeetName")]
    pub meet_name: String,
    #[serde(rename = "IDResult")]
    pub id_result: u64,
    #[serde(rename = "EventShort")]
    pub event_short: String,
    #[serde(rename = "Measure")]
    pub measure: String,
    #[serde(rename = "ResultDate")]
    pub result_date: String,
    #[serde(rename = "SeasonID")]
    pub season_id: i32,
    #[serde(rename = "Wind")]
    pub wind: Option<String>,
}

/// Top-level GetRankings response shape.
/// Preserves the original response JSON (`value`) for RFC 6901 pointer
/// evaluation (unknown fields, nested completeness, etc.).
///
/// `groupedRankings` is REQUIRED.
#[derive(Debug, Deserialize, Serialize)]
pub struct RawRankingsResponse {
    #[serde(rename = "groupedRankings")]
    pub grouped_rankings: Vec<Vec<RawRankingRecord>>,
    pub page: Option<u64>,
    pub complete: Option<serde_json::Value>,
    pub continuation: Option<RawContinuation>,
    /// Preserved original response for JSON pointer evaluation.
    #[serde(skip)]
    pub value: serde_json::Value,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RawContinuation {
    pub page: u64,
    pub complete: bool,
}

/// GetNavInfo response shape.
/// All fields use snake_case; serde `rename` maps from the API's PascalCase/camelCase keys.
///
/// `complete` is REQUIRED for valid pagination state.
#[derive(Debug, Deserialize, Serialize)]
pub struct RawNavInfoResponse {
    #[serde(rename = "state")]
    pub state: Option<RawNavState>,
    #[serde(rename = "event")]
    pub event: Option<RawNavEvent>,
    #[serde(rename = "divisions")]
    pub divisions: Option<Vec<RawNavDivision>>,
    #[serde(rename = "genders")]
    pub genders: Option<Vec<String>>,
    #[serde(rename = "complete")]
    pub complete: Option<bool>,
    #[serde(rename = "page")]
    pub page: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RawNavState {
    #[serde(rename = "StateID")]
    pub state_id: Option<u64>,
    #[serde(rename = "State")]
    pub state: Option<String>,
    #[serde(rename = "StateName")]
    pub state_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RawNavEvent {
    #[serde(rename = "EventShort")]
    pub event_short: Option<String>,
    #[serde(rename = "EventName")]
    pub event_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RawNavDivision {
    #[serde(rename = "DivisionID")]
    pub division_id: Option<u64>,
    #[serde(rename = "DivisionName")]
    pub division_name: Option<String>,
    #[serde(rename = "Indoor")]
    pub indoor: Option<bool>,
}
