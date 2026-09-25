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
    /// Operation ids this object has recorded, in the order they were first applied.
    ///
    /// Derived, not authoritative. What makes a repeat a no-op is the store's receipt, written in
    /// the same commit as the rows; this list is the object's own view of what it has recorded, for
    /// an operator reading `state`. A repeat names the operation it replayed and adds no second
    /// copy. The set grows unbounded but is bounded in practice by the 30-day
    /// `idempotency_retention` on this object: Restate purges state older than the retention.
    #[serde(default)]
    pub seen_operations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestRequest {
    /// Target table, e.g. `athletes`.
    pub table: String,
    /// Canonical entity observations; each row must carry its `id`.
    pub rows: Vec<Value>,
    /// The caller's stable name for this unit of work.
    ///
    /// Posting the same unit again — after a lost acknowledgement, or after re-reading it from a
    /// cache — must repeat the id, because the store's receipt is keyed by it: the second post then
    /// finds the rows already applied and appends nothing. A *different* unit must carry a
    /// different id: the same id with a different payload is refused, since the receipt standing
    /// under it describes other rows.
    pub operation_id: String,
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
    /// Store receipts the sweep's retention pass removed.
    ///
    /// A receipt is what makes a replayed append a no-op, so it is removed only once no invocation
    /// Restate still retains could replay the operation it names — see
    /// [`REPLAY_RETENTION_DAYS`](super::sweep::REPLAY_RETENTION_DAYS). This is the figure that says
    /// whether the store's receipt growth is being bounded at all.
    #[serde(default)]
    pub pruned_receipts: u64,
    /// Receipts the retention pass kept because it could not read a day off them.
    ///
    /// Non-zero means the policy has a blind spot an operator has to look at: a row the store cannot
    /// date cannot be shown to be outside its replay window, so it is left standing rather than
    /// removed.
    #[serde(default)]
    pub undated_receipts: u64,
}
