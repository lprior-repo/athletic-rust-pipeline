use std::collections::{HashMap, HashSet};

pub const ATHLETES_REQUIRED: &[&str] = &["Athlete ID", "Name", "School", "Graduation Year"];

pub const PERFORMANCES_REQUIRED: &[&str] = &[
    "Canonical Result ID",
    "Athlete ID",
    "Athlete",
    "School",
    "Graduation Year",
    "Meet ID",
    "Meet",
    "Date",
    "State",
    "Sport",
    "Event",
    "Mark",
    "Normalized Mark",
    "Timing",
    "Wind",
    "Round",
    "Place",
    "Source",
    "Source ResultID",
    "Source URL",
];

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

pub fn sheets_matching_prefix<'a>(
    sheets: &'a HashMap<String, Vec<Vec<String>>>,
    prefix: &str,
) -> Vec<&'a [String]> {
    let mut partitions: Vec<_> = sheets
        .iter()
        .filter(|(name, _)| name.starts_with(prefix))
        .collect();
    partitions.sort_unstable_by(|left, right| left.0.cmp(right.0));
    let mut all_rows = Vec::new();
    for (_, rows) in partitions {
        let skip = usize::from(!all_rows.is_empty());
        all_rows.extend(rows.iter().skip(skip).map(Vec::as_slice));
    }
    all_rows
}
