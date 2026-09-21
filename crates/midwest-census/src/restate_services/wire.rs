use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCount {
    pub table: String,
    pub rows: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusReply {
    pub tables: Vec<TableCount>,
    pub observations: u64,
    pub bytes_on_disk: u64,
    pub today: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConsolidateRequest {
    /// Tables to consolidate; empty means every table.
    #[serde(default)]
    pub tables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidatedTable {
    pub table: String,
    pub rows: usize,
    /// Consumer mailboxes withheld from the coaches snapshot by the contact contract.
    pub emails_withheld: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidateReply {
    pub tables: Vec<ConsolidatedTable>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReportRequest {
    /// `core` (default) or `all_sources`.
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportReply {
    pub scope: String,
    pub generated_on: String,
    /// Totals as the census document publishes them; the per-state breakdown stays in the JSON/CSV.
    pub totals: Value,
    pub json_path: String,
    pub csv_path: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BestsRequest {
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub grad_year: Option<i16>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestsReply {
    pub cohort: String,
    pub rows: usize,
    pub jsonl: String,
    pub csv: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorkbookRequest {
    #[serde(default)]
    pub grad_year: Option<i16>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbookReply {
    pub path: String,
    pub grad_year: Option<i16>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IngestState {
    /// Object key this state belongs to.
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRequest {
    /// Target table, e.g. `athletes`.
    pub table: String,
    /// Canonical entity observations; each row must carry its `id`.
    pub rows: Vec<Value>,
    /// Cursor the caller claims to have consumed; stored only after the append commits.
    #[serde(default)]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestReply {
    pub endpoint: String,
    /// Rows accepted in this request. `u64`, not `usize`: the wire shape must not change with the
    /// host pointer width.
    pub appended: u64,
    pub total_observations: u64,
    pub cursor: Option<String>,
    pub last_appended_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRequest {
    /// Window label being declared complete, e.g. `2026-W38`.
    pub window: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepRequest {
    /// Ingest object keys to observe.
    pub endpoints: Vec<String>,
    /// How many windows to observe before completing.
    #[serde(default = "default_windows")]
    pub windows: u32,
    /// Seconds slept between windows.
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
    /// Endpoints that have never accepted an observation.
    pub stale: Vec<String>,
    pub today: String,
    /// Where the durable pass wrote this report, when it reached that step.
    pub report_path: Option<String>,
}
