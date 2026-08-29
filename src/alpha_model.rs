use serde::{Deserialize, Serialize};

/// Authorization manifest loaded from the TOML `[authorization]` section.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthorizationConfig {
    pub enabled: bool,
    pub permission_reference: String,
    pub allowed_routes: Vec<String>,
    pub allowed_sports: Vec<String>,
    pub allowed_states: Vec<String>,
    pub allowed_seasons: Vec<i32>,
    pub allowed_genders: Vec<String>,
    pub allowed_fields: Vec<String>,
    pub allow_profile_enrichment: bool,
    pub max_concurrent_requests: usize,
    pub min_delay_ms: u64,
}

/// API connection and pagination settings from `[api]`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AlphaApiConfig {
    pub base_url: String,
    pub rankings_path: String,
    pub nav_info_path: String,
    pub timeout_seconds: u64,
    pub max_retries: usize,
    pub pagination: PaginationConfig,
}

/// Pagination strategy selected by the authorization manifest.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum PaginationConfig {
    SingleResponse {
        complete_pointer: String,
    },
    NextPage {
        has_more_pointer: String,
        next_page_pointer: String,
        request_page_key: String,
    },
}

/// A single matrix item the collector iterates.
#[derive(Debug, Clone)]
pub struct AlphaRequest {
    pub state_id: u64,
    pub season_id: i32,
    pub gender: String,
    pub event_short: String,
    pub continuation: Option<serde_json::Value>,
}

/// Normalized athlete record produced by the alpha pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceAthlete {
    pub athlete_id: u64,
    pub athlete_name: String,
    pub school: String,
    pub state: String,
    pub graduation_year: Option<i32>,
    pub cohort_evidence: String,
    pub gender: String,
    pub sport: String,
    pub profile_url: String,
    pub results: Vec<SourceResult>,
    pub source_urls: Vec<String>,
}

/// Normalized result record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceResult {
    pub result_id: Option<u64>,
    pub event: String,
    pub mark: String,
    pub season: String,
    pub date: String,
    pub meet_name: String,
    pub wind: Option<String>,
    pub result_url: Option<String>,
    pub source_url: String,
}

/// One row inside `groupedRankings` — the confirmed API shape.
///
/// `groupedRankings` is `Vec<Vec<RawRankingRecord>>`: outer array of groups,
/// inner array of athlete records per group.
#[derive(Debug, Deserialize)]
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
    #[serde(rename = "Results")]
    pub results: Vec<RawRankingResult>,
}

/// One result entry inside a `RawRankingRecord`.
#[derive(Debug, Deserialize)]
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
    #[serde(default)]
    pub wind: Option<String>,
    // Preserve any extra API fields for future use; ignore unknowns.
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

/// Top-level GetRankings response shape.
#[derive(Debug, Deserialize)]
pub struct RawRankingsResponse {
    #[serde(rename = "groupedRankings")]
    pub grouped_rankings: Vec<Vec<RawRankingRecord>>,
    pub page: Option<u64>,
    pub complete: Option<bool>,
    pub continuation: Option<RawContinuation>,
}

#[derive(Debug, Deserialize)]
pub struct RawContinuation {
    pub page: u64,
    pub complete: bool,
}

/// GetNavInfo response shape.
#[derive(Debug, Deserialize)]
pub struct RawNavInfoResponse {
    pub state: Option<RawNavState>,
    pub event: Option<RawNavEvent>,
    pub divisions: Option<Vec<RawNavDivision>>,
    pub genders: Option<Vec<String>>,
    pub complete: Option<bool>,
    pub page: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct RawNavState {
    #[serde(rename = "StateID")]
    pub state_id: Option<u64>,
    #[serde(rename = "State")]
    pub state: Option<String>,
    #[serde(rename = "StateName")]
    pub state_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawNavEvent {
    #[serde(rename = "EventShort")]
    pub event_short: Option<String>,
    #[serde(rename = "EventName")]
    pub event_name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct RawNavDivision {
    #[serde(rename = "DivisionID")]
    pub division_id: Option<u64>,
    #[serde(rename = "DivisionName")]
    pub division_name: Option<String>,
    #[serde(rename = "Indoor")]
    pub indoor: Option<bool>,
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn parse_rankings_fixture() {
        let fixture = std::fs::read_to_string(
            Path::new("fixtures/alpha/get-rankings-redacted.json"),
        )
        .expect("fixture must exist");
        let resp: RawRankingsResponse =
            serde_json::from_str(&fixture).expect("fixture must deserialize");

        // groupedRankings is present with correct nesting.
        assert_eq!(resp.grouped_rankings.len(), 2);
        assert_eq!(resp.grouped_rankings[0].len(), 2); // first group has 2 records.
        assert_eq!(resp.grouped_rankings[1].len(), 0); // second group is empty.

        // First record fields match fixture PascalCase keys.
        let rec = &resp.grouped_rankings[0][0];
        assert_eq!(rec.athlete_id, 90_000_001);
        assert_eq!(rec.athlete_name, "Test Runner");
        assert_eq!(rec.state, "TS");
        assert_eq!(rec.results.len(), 1);

        let r0 = &rec.results[0];
        assert_eq!(r0.measure, "10.50");
        assert_eq!(r0.wind, Some("+1.2".to_owned()));
        assert_eq!(r0.event_short, "100m");

        // Second record (same athlete, different event).
        let rec2 = &resp.grouped_rankings[0][1];
        assert_eq!(rec2.athlete_id, 90_000_001);
        assert_eq!(rec2.athlete_name, "Test Runner");
        assert_eq!(rec2.state, "TS");
        assert_eq!(rec2.results.len(), 1);
        assert_eq!(rec2.results[0].measure, "21.30");
        assert_eq!(rec2.results[0].event_short, "200m");
        assert_eq!(rec2.results[0].wind, None);

        // Continuation metadata present.
        assert_eq!(resp.continuation.as_ref().unwrap().page, 2);
        assert!(!resp.continuation.as_ref().unwrap().complete);
    }

    #[test]
    fn parse_nav_info_fixture() {
        let fixture = std::fs::read_to_string(
            Path::new("fixtures/alpha/get-nav-info-redacted.json"),
        )
        .expect("fixture must exist");
        let resp: RawNavInfoResponse =
            serde_json::from_str(&fixture).expect("fixture must deserialize");

        let state = resp.state.unwrap();
        assert_eq!(state.state_id, Some(90_000_001));
        assert_eq!(state.state, Some("TS".to_owned()));
        assert_eq!(state.state_name, Some("Test State".to_owned()));

        let event = resp.event.unwrap();
        assert_eq!(event.event_short, Some("100m".to_owned()));
        assert_eq!(event.event_name, Some("100 Meters".to_owned()));

        let div = resp.divisions.unwrap();
        assert_eq!(div.len(), 1);
        assert_eq!(div[0].division_id, Some(90_000_001));
        assert_eq!(div[0].division_name, Some("Test Division".to_owned()));
        assert_eq!(div[0].indoor, Some(false));

        assert_eq!(resp.genders, Some(vec!["m".to_owned()]));
        assert_eq!(resp.complete, Some(true));
        assert_eq!(resp.page, Some(1));
    }
}
