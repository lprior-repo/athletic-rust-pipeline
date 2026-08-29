use crate::alpha_model::RankingRecord;
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

impl RawRankingRecord {
    /// Convert to flat `RankingRecord` list.
    ///
    /// Uses nested `Results` when present and non-empty; falls back to
    /// flattened row fields when Results is absent or empty.
    ///
    /// Required nested fields: IDResult, EventShort, Measure, ResultDate, SeasonID,
    /// MeetID, MeetName.  Required flattened fields: IDResult, EventShort, Measure,
    /// ResultDate, SeasonID, MeetName, MeetID.
    pub fn to_flattened_records(&self) -> Result<Vec<RankingRecord>, String> {
        if let Some(ref results) = self.results {
            if !results.is_empty() {
                return self.from_nested_results(results);
            }
        }
        self.from_flattened()
    }

    fn from_nested_results(&self, results: &[RawRankingResult]) -> Result<Vec<RankingRecord>, String> {
        let mut records = Vec::new();
        for r in results {
            if r.id_result == 0 {
                return Err("RawRankingResult: missing required IDResult".into());
            }
            if r.event_short.is_empty() {
                return Err("RawRankingResult: missing required EventShort".into());
            }
            if r.measure.is_empty() {
                return Err("RawRankingResult: missing required Measure".into());
            }
            if r.result_date.is_empty() {
                return Err("RawRankingResult: missing required ResultDate".into());
            }
            if r.meet_id == 0 {
                return Err("RawRankingResult: missing required MeetID".into());
            }
            if r.meet_name.is_empty() {
                return Err("RawRankingResult: missing required MeetName".into());
            }
            if r.season_id == 0 {
                return Err("RawRankingResult: missing required SeasonID".into());
            }
            records.push(RankingRecord {
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
            });
        }
        Ok(records)
    }

    fn from_flattened(&self) -> Result<Vec<RankingRecord>, String> {
        let id_result = self.id_result
            .ok_or("RawRankingRecord flattened: missing required IDResult")?;
        if id_result == 0 {
            return Err("RawRankingRecord flattened: IDResult must not be zero".into());
        }
        let event_short = self.event_short.clone()
            .ok_or("RawRankingRecord flattened: missing required EventShort")?;
        if event_short.is_empty() {
            return Err("RawRankingRecord flattened: EventShort must not be empty".into());
        }
        let measure = self.measure.clone()
            .ok_or("RawRankingRecord flattened: missing required Measure")?;
        if measure.is_empty() {
            return Err("RawRankingRecord flattened: Measure must not be empty".into());
        }
        let result_date = self.result_date.clone()
            .ok_or("RawRankingRecord flattened: missing required ResultDate")?;
        if result_date.is_empty() {
            return Err("RawRankingRecord flattened: ResultDate must not be empty".into());
        }
        let season_id = self.season_id
            .ok_or("RawRankingRecord flattened: missing required SeasonID")?;
        if season_id == 0 {
            return Err("RawRankingRecord flattened: SeasonID must not be zero".into());
        }
        let meet_name = self.meet_name.clone()
            .ok_or("RawRankingRecord flattened: missing required MeetName")?;
        if meet_name.is_empty() {
            return Err("RawRankingRecord flattened: MeetName must not be empty".into());
        }
        let meet_id = self.meet_id
            .ok_or("RawRankingRecord flattened: missing required MeetID")?;
        if meet_id == 0 {
            return Err("RawRankingRecord flattened: MeetID must not be zero".into());
        }
        Ok(vec![RankingRecord {
            athlete_id: self.athlete_id,
            athlete_name: self.athlete_name.clone(),
            grade_id: self.grade_id,
            team_name: self.team_name.clone(),
            state: self.state.clone(),
            meet_id,
            meet_name,
            result_id: Some(id_result),
            event_short,
            measure,
            result_date,
            season_id,
            wind: self.wind.clone().flatten(),
        }])
    }
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

/// Manually deserialize to capture the full JSON value for pointer walking.
///
/// - Requires `groupedRankings` to be present.
/// - Rejects wrong type for groupedRankings, groups, or rows.
/// - Propagates RawRankingRecord / RawContinuation parse errors.
/// - No silent filtering — every malformed item is an error.
impl RawRankingsResponse {
    pub fn from_json(text: &str) -> Result<Self, String> {
        let value: serde_json::Value = serde_json::from_str(text)
            .map_err(|e| format!("JSON parse error: {e}"))?;

        let grouped_raw = value.get("groupedRankings")
            .ok_or("missing required field: groupedRankings")?;
        let groups_arr = grouped_raw.as_array()
            .ok_or("groupedRankings must be an array")?;

        let grouped_rankings: Result<Vec<Vec<RawRankingRecord>>, String> = groups_arr
            .iter()
            .map(|group| {
                let items = group.as_array()
                    .ok_or_else(|| "group inside groupedRankings must be an array".to_string())?;
                items.iter()
                    .map(|row| {
                        RawRankingRecord::deserialize(row.clone())
                            .map_err(|e| format!("malformed row: {e}"))
                    })
                    .collect()
            })
            .collect();

        let grouped_rankings = grouped_rankings?;
        // Strict page type: if present, must be u64 or explicit null; no silent drops.
        let page = match value.get("page") {
            Some(serde_json::Value::Null) => None,
            Some(serde_json::Value::Number(n)) if n.is_u64() => Some(n.as_u64().unwrap()),
            Some(v) => return Err(format!("page must be u64 or null, got: {v}")),
            None => None,
        };
        let complete = value.get("complete").cloned();
        let continuation = match value.get("continuation") {
            Some(serde_json::Value::Null) => None,
            Some(cont_raw) => Some(RawContinuation::deserialize(cont_raw.clone())
                .map_err(|e| format!("malformed continuation: {e}"))?),
            None => None,
        };

        Ok(RawRankingsResponse {
            grouped_rankings,
            page,
            complete,
            continuation,
            value,
        })
    }
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

impl RawNavInfoResponse {
    /// Validate the nav_info response shape.
    ///
    /// Requires the response to contain valid pagination metadata.
    /// Rejects responses missing both complete and page, or with
    /// wrong-type values for those fields.
    pub fn validate(&self) -> Result<(), &'static str> {
        match (&self.complete, &self.page) {
            (Some(_), _) | (None, Some(_)) => Ok(()),
            (None, None) => Err("RawNavInfoResponse: missing required complete or page field"),
        }
    }
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
