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
    pub allowed_profile_routes: Vec<String>,
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
    #[allow(dead_code)]
    pub season_id: i32,
    pub gender: String,
    pub event_short: String,
    pub indoor: bool,
    pub continuation: Option<serde_json::Value>,
}

/// Normalized athlete record produced by the alpha pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SourceAthlete {
    pub athlete_id: u64,
    pub athlete_name: String,
    pub grade_id: u64,
    pub team_name: String,
    pub state: String,
}

/// Normalized result record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SourceResult {
    pub result_id: u64,
    pub event_short: String,
    pub measure: String,
    pub result_date: String,
    pub season_id: i32,
}

/// Normalised ranking row produced by the alpha API client.
#[derive(Debug, Clone, Serialize)]
pub struct RankingRecord {
    pub athlete_id: u64,
    pub athlete_name: String,
    pub grade_id: u64,
    pub team_name: String,
    pub state: String,
    pub meet_id: u64,
    pub meet_name: String,
    pub result_id: Option<u64>,
    pub event_short: String,
    pub measure: String,
    pub result_date: String,
    pub season_id: i32,
    pub wind: Option<String>,
}
