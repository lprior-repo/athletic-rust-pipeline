//! The ingest object's wire types, and the sweeping observer that watches them.
//!
//! Split out of `wire.rs`, which had grown past the 300-line budget: these are the types of the
//! append-window family — one durable object per source endpoint, plus the sweep that samples every
//! endpoint's counters once a window. The census-side types stay in the parent module.

use serde::{Deserialize, Serialize};
use serde_json::Value;

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
