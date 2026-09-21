use crate::model::SourceRecord;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ExportRow {
    pub source: SourceRecord,
    pub extra_fields: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportSheetStats {
    pub name: String,
    pub rows: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportStats {
    pub sheets: Vec<ExportSheetStats>,
    pub aggregate_rows: u64,
    pub source_sha256: String,
}
