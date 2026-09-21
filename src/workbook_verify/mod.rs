mod digest;
mod headers;
mod sheet;
mod stream;
mod verify;

use serde::{Deserialize, Serialize};

pub use verify::verify_fields;

const MAX_EXCEL_ROW: u32 = 1_048_576;
const MAX_EXCEL_COLUMN: u32 = 16_384;
const GENERATED_SHEETS: [&str; 3] = ["Athletic Matches", "Corrections", "Summary"];

/// Aggregate evidence that an exported workbook preserves source fields.
/// This verifier does not prove any positive-match or decision rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationReport {
    pub expected_sha256: String,
    pub source_sha256_before: String,
    pub source_sha256_after: String,
    pub source_hash_before_matches: bool,
    pub source_hash_after_matches: bool,
    pub source_sheet_count: u64,
    pub output_sheet_count: u64,
    pub matched_sheet_count: u64,
    pub source_row_count: u64,
    pub output_row_count: u64,
    pub matched_row_count: u64,
    pub source_field_count: u64,
    pub output_field_count: u64,
    pub matched_field_count: u64,
    pub source_header_count: u64,
    pub output_header_count: u64,
    pub matched_header_count: u64,
    pub appended_header_count: u64,
}

#[derive(Debug, Default)]
struct SheetCounts {
    source_rows: u64,
    output_rows: u64,
    matched_rows: u64,
    source_fields: u64,
    output_fields: u64,
    matched_fields: u64,
    source_headers: u64,
    output_headers: u64,
    matched_headers: u64,
}
