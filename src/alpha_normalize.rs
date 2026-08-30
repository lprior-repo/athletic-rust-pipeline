use crate::alpha_model::{RankingRecord, SourceResult as ModelSourceResult};
/// Safe normalization for source records and athlete deduplication.
pub use crate::alpha_url::canonical_state;
use crate::alpha_url::{validate_profile_url, validate_result_url, validate_source_url};
use crate::marks;
use crate::model::Mark;
pub use crate::model::SourceRecord;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    pub profile_url: String,
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
    pub profile_url: String,
    pub results: Vec<ResultRecord>,
    pub source_urls: Vec<String>,
    pub exception_notes: Vec<String>,
}

pub fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn normalize_mark_entry(mark: Mark) -> Mark {
    let mut normalized = marks::normalize_mark(mark);
    if let Some(date) = normalized.date.split_whitespace().next() {
        normalized.date = date.to_owned();
    }
    normalized
}

/// Normalize a source record into a SourceAthlete.
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
        let n = normalize_whitespace(first);
        if let Some(last) = record.fields.get("last_name") {
            athlete.first_name = format!("{} {}", n, normalize_whitespace(last));
        } else {
            athlete.first_name = n;
        }
    }
    if let Some(last) = record.fields.get("last_name") {
        athlete.last_name = normalize_whitespace(last);
    }

    if let Some(school) = record.fields.get("school") {
        athlete.school = normalize_whitespace(school);
    }
    if let Some(state) = record.fields.get("state") {
        if let Some(canonical) = canonical_state(state) {
            athlete.state = canonical;
        } else {
            athlete
                .exception_notes
                .push(format!("unknown state '{}'", state.trim()));
        }
    }
    if let Some(city) = record.fields.get("city") {
        let t = city.trim();
        if !t.is_empty() && !t.contains(',') && !t.contains('\\') {
            athlete.city = normalize_whitespace(t);
        }
    }

    if let Some(profile) = record.fields.get("profile_url") {
        if let Some(valid) = validate_profile_url(profile) {
            athlete.profile_url = valid;
        }
    }

    if let Some(results) = record.fields.get("result_urls") {
        for url in results.split(';') {
            if let Some(valid) = validate_result_url(url.trim()) {
                if !valid.is_empty() && valid != athlete.profile_url {
                    if !athlete.results.iter().any(|r| r.result_url == valid) {
                        athlete.results.push(ResultRecord {
                            result_url: valid,
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }

    if let Some(source) = record.fields.get("source_url") {
        if let Some(valid) = validate_source_url(source) {
            athlete.source_urls = vec![valid];
        }
    }

    if let Some(marks_raw) = record.fields.get("marks") {
        for mark_str in marks_raw.split(';') {
            let trimmed = mark_str.trim();
            if trimmed.is_empty() {
                continue;
            }
            let parts: Vec<&str> = trimmed.split('|').collect();
            if parts.len() >= 4 {
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
                let normalized = normalize_mark_entry(mark);
                if normalized.valid {
                    athlete.results.push(ResultRecord {
                        event: normalized.event,
                        mark: normalized.mark,
                        season: normalized.season,
                        date: normalized.date,
                        meet_name: normalized.meet_name,
                        wind: normalized.wind,
                        profile_url: athlete.profile_url.clone(),
                        ..Default::default()
                    });
                }
            }
        }
    }

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

/// Convert a RankingRecord into a ResultRecord.
pub fn from_ranking_record(rec: &RankingRecord, profile_url: &str) -> ResultRecord {
    ResultRecord {
        profile_url: profile_url.to_owned(),
        source_url: format!("https://athletic.net/athlete/{}", rec.athlete_id),
        ..Default::default()
    }
}

/// Convert a model SourceResult into a ResultRecord.
pub fn from_model_source_result(sr: &ModelSourceResult, profile_url: &str) -> ResultRecord {
    ResultRecord {
        event: sr.event_short.clone(),
        mark: sr.measure.clone(),
        date: sr.result_date.clone(),
        season: sr.season_id.to_string(),
        profile_url: profile_url.to_owned(),
        result_id: sr.result_id,
        ..Default::default()
    }
}

/// Dedup athletes by athlete_id, keeping first occurrence.
pub fn dedup_athletes(athletes: Vec<SourceAthlete>) -> Vec<SourceAthlete> {
    let mut seen = BTreeMap::new();
    let mut out = Vec::new();
    for a in athletes {
        let id = a.athlete_id;
        if !seen.contains_key(&id) && (id != 0 || !a.exception_notes.is_empty()) {
            seen.insert(id, out.len());
            out.push(a);
        }
    }
    out
}

/// Merge a new SourceAthlete into an existing map by athlete_id.
pub fn merge_athlete(map: &mut BTreeMap<u64, SourceAthlete>, new: SourceAthlete) -> u64 {
    let id = new.athlete_id;
    if id == 0 {
        let note = "athlete_id missing or zero; cannot deduplicate";
        let exn = SourceAthlete {
            athlete_id: 0,
            exception_notes: vec![note.to_owned()],
            ..new
        };
        let existing = map.entry(0).or_default();
        for n in exn.exception_notes {
            if !existing.exception_notes.contains(&n) {
                existing.exception_notes.push(n);
            }
        }
        return 0;
    }

    match map.get_mut(&id) {
        Some(existing) => {
            for nr in &new.results {
                let mut copy = nr.clone();
                copy.profile_url = String::new();
                if copy.result_url.is_empty() && copy.result_id == 0 {
                    continue;
                }
                let is_dup = existing.results.iter().any(|r| {
                    if copy.result_id != 0 && copy.result_id == r.result_id {
                        return true;
                    }
                    if copy.result_id == 0 && r.result_id == 0 {
                        return copy.event == r.event
                            && copy.mark == r.mark
                            && copy.date == r.date
                            && copy.meet_name == r.meet_name
                            && copy.source_url == r.source_url;
                    }
                    false
                });
                if !is_dup {
                    existing.results.push(copy);
                }
            }

            for n in &new.results {
                if !n.result_url.is_empty()
                    && !existing
                        .results
                        .iter()
                        .any(|r| r.result_url == n.result_url)
                {
                    existing.results.push(n.clone());
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
            if existing.city.is_empty() && !new.city.is_empty() {
                existing.city = new.city;
            }

            for url in new.source_urls {
                if !existing.source_urls.contains(&url) {
                    existing.source_urls.push(url);
                }
            }
            for note in new.exception_notes {
                if !existing.exception_notes.contains(&note) {
                    existing.exception_notes.push(note);
                }
            }
        }
        None => {
            map.insert(id, new);
        }
    }

    id
}
