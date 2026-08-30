/// Safe normalization for source records and athlete deduplication.
pub use crate::model::SourceRecord;
use crate::marks;
use crate::model::Mark;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use url::Url;

const ATHLETIC_NET_HOST: &str = "athletic.net";

fn allowed_source_hosts() -> &'static [&'static str] {
    &["athletic.net"]
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceAthlete {
    pub athlete_id: u64,
    pub first_name: String,
    pub last_name: String,
    pub school: String,
    pub state: String,
    pub location: String,
    pub profile_url: String,
    pub result_urls: Vec<String>,
    pub source_urls: Vec<String>,
    pub marks: Vec<Mark>,
    pub exception_notes: Vec<String>,
}

pub fn validate_url(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parsed = Url::parse(trimmed).ok()?;
    if parsed.scheme() != "https" {
        return None;
    }
    let host = parsed.host_str()?;
    if !allowed_source_hosts().contains(&host) {
        return None;
    }
    Some(trimmed.to_owned())
}

pub fn construct_profile_url(athlete_id: u64) -> Option<String> {
    if athlete_id == 0 {
        return None;
    }
    Some(format!("https://{}/athlete/{}", ATHLETIC_NET_HOST, athlete_id))
}

pub fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn canonical_state(s: &str) -> String {
    s.trim().to_uppercase()
}

#[allow(dead_code)]
pub fn normalize_mark_entry(mark: Mark) -> Mark {
    marks::normalize_mark(mark)
}

pub fn normalize_record(record: &SourceRecord) -> SourceAthlete {
    let mut athlete = SourceAthlete::default();

    if let Some(id_str) = record.fields.get("athlete_id") {
        if let Ok(id) = id_str.trim().parse::<u64>() {
            if id != 0 {
                athlete.athlete_id = id;
            }
        }
    }
    if let Some(first) = record.fields.get("first_name") {
        let normalized = normalize_whitespace(first);
        if let Some(last) = record.fields.get("last_name") {
            athlete.first_name = format!("{} {}", normalized, normalize_whitespace(last));
        } else {
            athlete.first_name = normalized;
        }
    }
    if let Some(last) = record.fields.get("last_name") {
        athlete.last_name = normalize_whitespace(last);
    }
    if let Some(school) = record.fields.get("school") {
        athlete.school = normalize_whitespace(school);
    }
    if let Some(state) = record.fields.get("state") {
        athlete.state = canonical_state(state);
    }
    if let Some(loc) = record.fields.get("location") {
        athlete.location = normalize_whitespace(loc);
    }
    if let Some(profile) = record.fields.get("profile_url") {
        if let Some(valid) = validate_url(profile) {
            athlete.profile_url = valid;
        }
    }
    if let Some(results) = record.fields.get("result_urls") {
        let urls: Vec<String> = results
            .split(';')
            .filter_map(|u| validate_url(u.trim()))
            .collect();
        if !urls.is_empty() {
            athlete.result_urls = urls;
        }
    }
    if let Some(source) = record.fields.get("source_url") {
        if let Some(valid) = validate_url(source) {
            athlete.source_urls = vec![valid];
        }
    }

    athlete
}

pub fn merge_athlete(
    map: &mut BTreeMap<u64, SourceAthlete>,
    new: SourceAthlete,
) -> u64 {
    let id = new.athlete_id;
    if id == 0 {
        return id;
    }

    match map.get_mut(&id) {
        Some(existing) => {
            for mark in new.marks {
                let is_dup = existing.marks.iter().any(|m| {
                    m.event == mark.event
                        && m.mark == mark.mark
                        && m.date == mark.date
                        && m.meet_name == mark.meet_name
                        && m.source_url == mark.source_url
                });
                if !is_dup {
                    existing.marks.push(mark);
                }
            }
            for url in new.result_urls {
                if !existing.result_urls.contains(&url) {
                    existing.result_urls.push(url);
                }
            }
            if !existing.profile_url.is_empty()
                && existing.profile_url != new.profile_url
                && !existing.result_urls.contains(&new.profile_url)
            {
                existing.result_urls.push(new.profile_url);
            }
            for url in new.source_urls {
                if !existing.source_urls.contains(&url) {
                    existing.source_urls.push(url);
                }
            }
            if !new.first_name.is_empty()
                && !existing.first_name.is_empty()
                && new.first_name != existing.first_name
            {
                existing.exception_notes.push(format!(
                    "first_name conflict: '{}' vs '{}'",
                    existing.first_name, new.first_name
                ));
            }
            if !new.last_name.is_empty()
                && !existing.last_name.is_empty()
                && new.last_name != existing.last_name
            {
                existing.exception_notes.push(format!(
                    "last_name conflict: '{}' vs '{}'",
                    existing.last_name, new.last_name
                ));
            }
            if !new.school.is_empty()
                && !existing.school.is_empty()
                && new.school != existing.school
            {
                existing.exception_notes.push(format!(
                    "school conflict: '{}' vs '{}'",
                    existing.school, new.school
                ));
            }
            if existing.state.is_empty() && !new.state.is_empty() {
                existing.state = new.state;
            }
            if existing.location.is_empty() && !new.location.is_empty() {
                existing.location = new.location;
            }
            existing.exception_notes.extend(new.exception_notes);
        }
        None => {
            map.insert(id, new);
        }
    }

    id
}

pub fn exception_for_missing_id(record: &SourceRecord) -> SourceAthlete {
    let mut athlete = SourceAthlete::default();
    athlete.exception_notes = vec![
        "athlete_id missing or zero; cannot deduplicate".to_owned(),
    ];
    if let Some(first) = record.fields.get("first_name") {
        let normalized = normalize_whitespace(first);
        if let Some(last) = record.fields.get("last_name") {
            athlete.first_name = format!("{} {}", normalized, normalize_whitespace(last));
        } else {
            athlete.first_name = normalized;
        }
    }
    if let Some(last) = record.fields.get("last_name") {
        athlete.last_name = normalize_whitespace(last);
    }
    if let Some(school) = record.fields.get("school") {
        athlete.school = normalize_whitespace(school);
    }
    if let Some(state) = record.fields.get("state") {
        athlete.state = canonical_state(state);
    }
    athlete
}
