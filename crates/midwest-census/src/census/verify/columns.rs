//! Column mapping helpers: header validation and prefix-based sheet collection.

use std::collections::{HashMap, HashSet};

/// Required columns for the Athletes sheet.
pub const ATHLETES_REQUIRED: &[&str] = &["Athlete ID", "Name", "School", "Graduation Year"];

/// Required columns for the Performances sheets.
pub const PERFORMANCES_REQUIRED: &[&str] = &["Athlete ID", "Event", "Mark"];

/// Check that a header row contains every required column, returning the missing ones.
pub fn missing_columns(headers: &[String], required: &[&str]) -> Vec<String> {
    let header_set: HashSet<&str> = headers.iter().map(|h| h.as_str()).collect();
    required
        .iter()
        .filter(|name| !header_set.contains(*name))
        .map(|name| (*name).to_string())
        .collect()
}

/// Build a column index map from a header row.
pub fn column_index(headers: &[String], name: &str) -> Option<usize> {
    headers.iter().position(|h| h.as_str() == name)
}

/// Collect all data rows from sheets whose name starts with `prefix`.
///
/// Returns every row from every matching sheet, in sheet order, with each sheet's data
/// appended sequentially (including header rows — the caller should strip them).
pub fn sheets_matching_prefix(
    sheets: &HashMap<String, Vec<Vec<String>>>,
    prefix: &str,
) -> Vec<Vec<String>> {
    let mut all_rows: Vec<Vec<String>> = Vec::new();
    for (key, rows) in sheets.iter() {
        if key.starts_with(prefix) {
            all_rows.extend(rows.iter().cloned());
        }
    }
    all_rows
}
