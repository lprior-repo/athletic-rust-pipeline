use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MAX_OPERATION_ID_BYTES: usize = 128;
pub const MAX_WINDOW_LABEL_BYTES: usize = 128;
pub const WINDOW_LABEL_RING: usize = 64;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IngestState {
    #[serde(default)]
    pub endpoint: String,
    #[serde(default)]
    pub total_observations: u64,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub last_appended_at: Option<String>,
    #[serde(default)]
    pub windows: Vec<String>,
    #[serde(default)]
    pub windows_completed: u64,
}

impl IngestState {
    pub fn completed_windows(&self) -> u64 {
        self.windows_completed
            .max(u64::try_from(self.windows.len()).map_or(u64::MAX, core::convert::identity))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRequest {
    pub table: String,
    pub rows: Vec<Value>,
    pub operation_id: String,
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordedIngestRequest {
    pub recorded: census_crawl::Recorded,
    pub operation_id: String,
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestReply {
    pub endpoint: String,
    pub appended: u64,
    #[serde(default)]
    pub written: u64,
    pub total_observations: u64,
    pub cursor: Option<String>,
    pub last_appended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRequest {
    pub window: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepRequest {
    pub endpoints: Vec<String>,
    #[serde(default = "default_windows")]
    pub windows: u32,
    #[serde(default = "default_window_seconds")]
    pub window_seconds: u64,
}

fn default_windows() -> u32 {
    1
}

fn default_window_seconds() -> u64 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointObservation {
    pub endpoint: String,
    pub total_observations: u64,
    pub cursor: Option<String>,
    pub completed_windows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepReport {
    pub windows_observed: u32,
    pub interrupted: bool,
    pub endpoints: Vec<EndpointObservation>,
    pub stale: Vec<String>,
    pub today: String,
    pub report_path: Option<String>,
    #[serde(default)]
    pub pruned_receipts: u64,
    #[serde(default)]
    pub undated_receipts: u64,
}
