use std::collections::{HashMap, HashSet};

pub const ATHLETES_REQUIRED: &[&str] = &["Athlete ID", "Name", "School", "Graduation Year"];

pub const PERFORMANCES_REQUIRED: &[&str] = &["Athlete ID", "Event", "Mark"];

pub fn missing_columns(headers: &[String], required: &[&str]) -> Vec<String> {
    let header_set: HashSet<&str> = headers.iter().map(|h| h.as_str()).collect();
    required
        .iter()
        .filter(|name| !header_set.contains(*name))
        .map(|name| (*name).to_string())
        .collect()
}

pub fn column_index(headers: &[String], name: &str) -> Option<usize> {
    headers.iter().position(|h| h.as_str() == name)
}

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
