use serde::{Deserialize, Serialize};

use census_domain::school_directory::{ChangeSet, IdentifiedKey, UpdateDecision};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub manifest_digest: String,
    pub now: Option<String>,
    pub lanes: Vec<LaneReport>,
    pub corpus: CorpusReport,
    pub changes: Option<ChangeReport>,
    pub schedule: Vec<SourceDecision>,
    pub outputs: Vec<String>,
    pub phases: Option<PhaseReport>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PhaseReport {
    pub geocode_requested: bool,
    pub validation_requested: bool,
    pub geocoded: usize,
    pub geocode_skipped: usize,
    pub geocode_empty: usize,
    pub geocode_refused: usize,
    pub geocode_unusable: usize,
    pub geocode_transport: usize,
    pub validated: usize,
    pub validation_skipped: usize,
    pub validation_rejected: usize,
    pub validation_unparsed: usize,
    pub validation_transport: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaneReport {
    pub source: String,
    pub path: String,
    pub sha256: String,
    pub entries: usize,
    pub skipped: usize,
    pub notes: usize,
    pub skipped_rows: Vec<LedgerRow>,
    pub note_rows: Vec<LedgerRow>,
    #[serde(default)]
    pub captured: Vec<IdentifiedKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerRow {
    pub line: usize,
    pub field: String,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CorpusReport {
    pub rows: usize,
    pub entries: usize,
    pub skipped: usize,
    pub notes: usize,
    pub merges: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ChangeReport {
    pub added: usize,
    pub removed: usize,
    pub modified: usize,
}

impl ChangeReport {
    pub fn of(changes: &ChangeSet) -> Self {
        Self {
            added: changes.added.len(),
            removed: changes.removed.len(),
            modified: changes.modified.len(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDecision {
    pub source: String,
    pub decision: String,
    pub due: String,
}

impl SourceDecision {
    pub fn label(decision: UpdateDecision) -> &'static str {
        match decision {
            UpdateDecision::Due { .. } => "due",
            UpdateDecision::NotDue { .. } => "not-due",
        }
    }
}
