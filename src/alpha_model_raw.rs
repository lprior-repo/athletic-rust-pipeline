use crate::alpha_model::RankingRecord;
use serde::{Deserialize, Serialize};

/// One row inside `groupedRankings` — supports both nested `Results` and
/// confirmed flattened row shapes. Rejects unknown fields.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
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
    #[serde(rename = "Results", default)]
    pub results: Vec<RawRankingResult>,
    // Flattened row fields.
    #[serde(rename = "IDResult", default)]
    pub id_result: Option<u64>,
    #[serde(rename = "EventShort", default)]
    pub event_short: Option<String>,
    #[serde(rename = "Measure", default)]
    pub measure: Option<String>,
    #[serde(rename = "ResultDate", default)]
    pub result_date: Option<String>,
    #[serde(rename = "SeasonID", default)]
    pub season_id: Option<i32>,
    #[serde(rename = "Wind", default)]
    pub wind: Option<Option<String>>,
    #[serde(rename = "MeetID", default)]
    pub meet_id: Option<u64>,
    #[serde(rename = "MeetName", default)]
    pub meet_name: Option<String>,
}

impl RawRankingRecord {
    /// Convert to flat `RankingRecord` list.
    ///
    /// Uses nested `Results` when present; falls back to the flattened
    /// row fields (one result per row).  Fails closed: missing required
    /// flattened fields are never fabricated as defaults.
    pub fn to_flattened_records(&self) -> Vec<RankingRecord> {
        if !self.results.is_empty() {
            self.results.iter().filter_map(|r| {
                // Reject malformed nested rows missing required fields.
                if r.id_result == 0 || r.event_short.is_empty() || r.measure.is_empty() || r.result_date.is_empty() {
                    return None;
                }
                Some(RankingRecord {
                    athlete_id: self.athlete_id,
                    athlete_name: self.athlete_name.clone(),
                    grade_id: self.grade_id,
                    team_name: self.team_name.clone(),
                    state: self.state.clone(),
                    meet_id: r.meet_id,
                    meet_name: r.meet_name.clone(),
                    result_id: Some(r.id_result),
                    event_short: r.event_short.clone(),
                    measure: r.measure.clone(),
                    result_date: r.result_date.clone(),
                    season_id: r.season_id,
                    wind: r.wind.clone(),
                })
            }).collect()
        } else if self.event_short.is_some() {
            // Flattened row: fail closed on missing required fields.
            if self.id_result.is_none() || self.measure.is_none()
                || self.result_date.is_none() || self.season_id.is_none()
            {
                return vec![];
            }
            self.event_short.as_ref().map(|es| RankingRecord {
                athlete_id: self.athlete_id,
                athlete_name: self.athlete_name.clone(),
                grade_id: self.grade_id,
                team_name: self.team_name.clone(),
                state: self.state.clone(),
                meet_id: self.meet_id.unwrap_or(0),
                meet_name: self.meet_name.clone().unwrap_or_default(),
                result_id: self.id_result,
                event_short: es.clone(),
                measure: self.measure.clone().unwrap_or_default(),
                result_date: self.result_date.clone().unwrap_or_default(),
                season_id: self.season_id.unwrap_or(0),
                wind: self.wind.clone().flatten(),
            }).into_iter().collect()
        } else {
            vec![]
        }
    }
}

/// One result entry inside a `RawRankingRecord`.
/// `IDResult` defaults to 0 when missing, enabling malformed row rejection
/// in `to_flattened_records()`.
#[derive(Debug, Deserialize, Serialize)]
pub struct RawRankingResult {
    #[serde(rename = "MeetID")]
    pub meet_id: u64,
    #[serde(rename = "MeetName")]
    pub meet_name: String,
    #[serde(rename = "IDResult", default)]
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
    #[serde(default)]
    pub wind: Option<String>,
}

/// Top-level GetRankings response shape.
/// Preserves the original response JSON (`value`) for RFC 6901 pointer
/// evaluation (unknown fields, nested completeness, etc.).
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

/// Manually deserialize to capture the full JSON value for pointer walking.
impl RawRankingsResponse {
    pub fn from_json(text: &str) -> Result<Self, serde_json::Error> {
        let value: serde_json::Value = serde_json::from_str(text)?;
        let grouped_rankings = value.get("groupedRankings")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter().filter_map(|g| {
                    g.as_array().map(|items| {
                        items.iter().filter_map(|r| {
                            RawRankingRecord::deserialize(r.clone()).ok()
                        }).collect::<Vec<_>>()
                    })
                }).collect()
            })
            .unwrap_or_default();
        let page = value.get("page").and_then(|v| v.as_u64());
        let complete = value.get("complete").cloned();
        let continuation = value.get("continuation")
            .and_then(|v| RawContinuation::deserialize(v.clone()).ok());
        Ok(RawRankingsResponse {
            grouped_rankings,
            page,
            complete,
            continuation,
            value,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RawContinuation {
    pub page: u64,
    pub complete: bool,
}

/// GetNavInfo response shape.
/// All fields use snake_case; serde `rename` maps from the API's camelCase keys.
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
