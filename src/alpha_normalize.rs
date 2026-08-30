/// Safe normalization for source records and athlete deduplication.
pub use crate::alpha_url::canonical_state;
pub use crate::model::SourceRecord;
use crate::alpha_url::{validate_profile_url, validate_result_url, validate_source_url};
use crate::marks;
use crate::model::Mark;
use serde::{Deserialize, Serialize};

/// A single result record with full metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResultRecord {
    pub result_id: u64,
    pub event: String,
    pub mark: String,
    pub season: String,
    pub date: String,
    pub meet_name: String,
    pub wind: Option<String>,
    pub source_url: String,
    pub result_url: String,
}

/// Normalized athlete record.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceAthlete {
    pub athlete_id: u64,
    pub first_name: String,
    pub last_name: String,
    pub school: String,
    pub state: String,
    pub city: String,
    pub profile_urls: Vec<String>,
    pub results: Vec<ResultRecord>,
    pub source_urls: Vec<String>,
    pub exception_notes: Vec<String>,
}

pub fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse a city/state field. Accepts "City, ST" or bare city name.
/// Rejects digits, contact/address text.
pub fn parse_location(raw: &str) -> Option<(String, String)> {
    let t = raw.trim();
    if t.is_empty() {
        return None;
    }
    let clean = t.replace(['-', '(', ')', '.'], "");
    if clean.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if clean.chars().next().map_or(false, |c| c.is_ascii_digit()) {
        return None;
    }
    // Try "City, ST" format
    if let Some((city, state)) = t.split_once(',') {
        let city = normalize_whitespace(city);
        let state = canonical_state(state);
        if !city.is_empty() {
            return Some((city, state.unwrap_or_default()));
        }
    }
    // Bare city — no state
    Some((normalize_whitespace(t), String::new()))
}

/// Parse and normalize a single mark string into a ResultRecord.
fn parse_mark_entry(
    mark_str: &str,
    _profile_url: &str,
    source_url: &str,
    result_id: Option<u64>,
) -> Option<ResultRecord> {
    let parts: Vec<&str> = mark_str.split('|').collect();
    if parts.len() < 4 {
        return None;
    }
    let mut mark = Mark::default();
    mark.event = parts[0].to_owned();
    mark.mark = parts[1].to_owned();
    mark.season = parts[2].to_owned();
    mark.date = parts[3].to_owned();
    if parts.len() > 4 {
        mark.meet_name = parts[4].to_owned();
    }
    if parts.len() > 5 {
        mark.wind = Some(parts[5].to_owned());
    }
    let normalized = marks::normalize_mark(mark);
    if !normalized.valid {
        return None;
    }
    Some(ResultRecord {
        event: normalized.canonical_event,
        mark: normalized.mark,
        season: normalized.season,
        date: normalized.date,
        meet_name: normalized.meet_name,
        wind: normalized.wind,
        result_id: result_id.unwrap_or(0),
        source_url: source_url.to_owned(),
        ..Default::default()
    })
}

/// Normalize a source record into a SourceAthlete.
pub fn normalize_record(record: &SourceRecord) -> SourceAthlete {
    let mut athlete = SourceAthlete::default();

    // Extract athlete_id
    if let Some(id_str) = record.fields.get("athlete_id") {
        if let Ok(id) = id_str.trim().parse::<u64>() {
            athlete.athlete_id = id;
        }
    }

    // Extract name fields separately
    if let Some(first) = record.fields.get("first_name") {
        athlete.first_name = normalize_whitespace(first);
    }
    if let Some(last) = record.fields.get("last_name") {
        athlete.last_name = normalize_whitespace(last);
    }

    if let Some(school) = record.fields.get("school") {
        athlete.school = normalize_whitespace(school);
    }

    // Extract state
    if let Some(state) = record.fields.get("state") {
        if let Some(canonical) = canonical_state(state) {
            athlete.state = canonical;
        } else {
            athlete.exception_notes.push(format!("unknown state '{}'", state.trim()));
        }
    }

    // Extract city (validated — no free-form address text)
    if let Some(city) = record.fields.get("city") {
        if let Some((city_str, _)) = parse_location(city) {
            athlete.city = city_str;
        }
    }

    // Validate and collect profile URLs
    if let Some(profile) = record.fields.get("profile_url") {
        for url in profile.split(';') {
            if let Some(valid) = validate_profile_url(url) {
                if !athlete.profile_urls.contains(&valid) {
                    athlete.profile_urls.push(valid);
                }
            }
        }
    }

    // Collect result URLs
    if let Some(results) = record.fields.get("result_urls") {
        for url in results.split(';') {
            if let Some(valid) = validate_result_url(url.trim()) {
                if !athlete.profile_urls.contains(&valid)
                    && !athlete.results.iter().any(|r| r.result_url == valid)
                {
                    athlete.results.push(ResultRecord {
                        result_url: valid,
                        ..Default::default()
                    });
                }
            }
        }
    }

    // Collect source URLs
    if let Some(source) = record.fields.get("source_url") {
        for url in source.split(';') {
            if let Some(valid) = validate_source_url(url) {
                if !athlete.source_urls.contains(&valid) {
                    athlete.source_urls.push(valid);
                }
            }
        }
    }

    // Parse marks data and normalize with marks module
    if let Some(marks_raw) = record.fields.get("marks") {
        let source_url = athlete.source_urls.first().cloned().unwrap_or_default();
        for mark_str in marks_raw.split(';') {
            let trimmed = mark_str.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(rr) = parse_mark_entry(trimmed, "", &source_url, None) {
                athlete.results.push(rr);
            }
        }
    }

    // Attach result IDs for dedup
    if let Some(result_ids) = record.fields.get("result_ids") {
        let ids: Vec<u64> = result_ids
            .split(';')
            .filter_map(|s| s.trim().parse::<u64>().ok())
            .collect();
        for (i, rid) in ids.iter().enumerate() {
            if i < athlete.results.len() {
                athlete.results[i].result_id = *rid;
            }
        }
    }

    athlete
}
