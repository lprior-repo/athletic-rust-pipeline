use crate::Table;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordedBatch {
    pub table: Table,
    pub rows: Vec<Value>,
}

impl RecordedBatch {
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecordedJournal {
    pub phase: String,
    pub key: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Recorded {
    pub rows: Vec<RecordedBatch>,
    pub journal: Vec<RecordedJournal>,
    #[serde(default)]
    pub effects: Vec<RecordedEffect>,
}

impl Recorded {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty() && self.journal.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedEffect {
    pub rows: Range<usize>,
    pub journal: Range<usize>,
    pub witness: usize,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FragmentManifest {
    pub digest: String,
    pub encoded_bytes: usize,
    pub rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct FragmentSummary {
    pub fragments: usize,
    pub digest: String,
    pub encoded_bytes: usize,
    pub rows: usize,
}
