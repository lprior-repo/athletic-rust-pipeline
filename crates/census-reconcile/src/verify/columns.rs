use std::collections::HashSet;

pub const ATHLETES_REQUIRED: &[&str] = &["Athlete ID", "Name", "School", "Graduation Year"];
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
