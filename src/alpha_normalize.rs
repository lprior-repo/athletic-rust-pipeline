/// Safe normalization for source records and athlete deduplication.
pub use crate::alpha_model::{SourceAthlete, SourceResult};
pub use crate::alpha_url::canonical_state;
pub use crate::model::SourceRecord;
pub type ResultRecord = SourceResult;

use crate::alpha_url::{validate_profile_url, validate_result_url, validate_source_url};
use url::Url;
pub use crate::alpha_normalize_helpers::parse_location;
use crate::alpha_normalize_helpers::parse_mark_entry;

pub fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}


fn note(athlete: &mut SourceAthlete, message: &str) {
    if !athlete.exception_notes.iter().any(|existing| existing == message) {
        athlete.exception_notes.push(message.to_owned());
    }
}

fn parse_id(value: Option<&String>) -> Option<u64> {
    let raw = value?.trim();
    let id = raw.parse::<u64>().ok()?;
    (id > 0).then_some(id)
}

fn profile_id(url: &str) -> Option<u64> {
    let parsed = Url::parse(url).ok()?;
    let value = parsed.path().strip_prefix("/athlete/")?;
    value.parse::<u64>().ok().filter(|id| *id > 0)
}

fn result_id(url: &str) -> Option<u64> {
    let parsed = Url::parse(url).ok()?;
    let value = parsed.path().strip_prefix("/result/")?;
    value.parse::<u64>().ok().filter(|id| *id > 0)
}

fn parsed_ids(raw: &str, athlete: &mut SourceAthlete) -> Vec<Option<u64>> {
    raw.split(';')
        .map(|token| {
            let trimmed = token.trim();
            if trimmed.is_empty() {
                return None;
            }
            let parsed = trimmed.parse::<u64>().ok().filter(|id| *id > 0);
            if parsed.is_none() {
                note(athlete, "invalid result ID evidence");
            }
            parsed
        })
        .collect()
}

/// Normalize a source record into the canonical alpha model type.
pub fn normalize_record(record: &SourceRecord) -> SourceAthlete {
    let mut athlete = SourceAthlete::default();
    athlete.athlete_id = match parse_id(record.fields.get("athlete_id")) {
        Some(id) => id,
        None => 0,
    };
    if athlete.athlete_id == 0 {
        note(&mut athlete, "athlete_id missing or invalid");
    }
    athlete.first_name = record.fields.get("first_name")
        .map_or_else(String::new, |value| normalize_whitespace(value));
    athlete.last_name = record.fields.get("last_name")
        .map_or_else(String::new, |value| normalize_whitespace(value));
    athlete.athlete_name = record.fields.get("athlete_name")
        .map_or_else(String::new, |value| normalize_whitespace(value));
    if athlete.athlete_name.is_empty() {
        athlete.athlete_name = format!("{} {}", athlete.first_name, athlete.last_name)
            .trim().to_owned();
    }
    athlete.school = record.fields.get("school")
        .map_or_else(String::new, |value| normalize_whitespace(value));
    athlete.team_name = record.fields.get("team_name")
        .map_or_else(String::new, |value| normalize_whitespace(value));
    athlete.gender = record.fields.get("gender")
        .map_or_else(String::new, |value| normalize_whitespace(value));
    athlete.sport = record.fields.get("sport")
        .map_or_else(String::new, |value| normalize_whitespace(value));

    if let Some(state) = record.fields.get("state") {
        match canonical_state(state) {
            Some(canonical) => athlete.state = canonical,
            None if !state.trim().is_empty() => note(&mut athlete, "unknown state evidence"),
            None => {}
        }
    }
    if let Some(city) = record.fields.get("city") {
        match parse_location(city) {
            Some((city_name, location_state)) => {
                athlete.city = city_name;
                if athlete.state.is_empty() {
                    athlete.state = location_state;
                } else if !location_state.is_empty() && athlete.state != location_state {
                    note(&mut athlete, "city/state evidence conflicts with state field");
                }
            }
            None => note(&mut athlete, "invalid city/location evidence"),
        }
    }

    if let Some(grade) = record.fields.get("grade_id") {
        match grade.trim().parse::<u64>() {
            Ok(value) if value > 0 => athlete.grade_id = value,
            Ok(_) | Err(_) if !grade.trim().is_empty() => note(&mut athlete, "invalid grade evidence"),
            _ => {}
        }
    }
    if let Some(year) = record.fields.get("graduation_year") {
        match year.trim().parse::<i32>() {
            Ok(value) => athlete.graduation_year = Some(value),
            Err(_) if !year.trim().is_empty() => note(&mut athlete, "invalid graduation-year evidence"),
            _ => {}
        }
    }
    athlete.cohort_evidence = match athlete.graduation_year {
        Some(year) => format!("graduation_year={year}"),
        None => record.fields.get("cohort_evidence")
            .map_or_else(String::new, |value| normalize_whitespace(value)),
    };
    if let Some(profile_field) = record.fields.get("profile_url") {
        for raw_url in profile_field.split(';').map(str::trim).filter(|url| !url.is_empty()) {
            let Some(valid) = validate_profile_url(raw_url) else {
                note(&mut athlete, "invalid profile URL evidence");
                continue;
            };
            if profile_id(&valid).is_some_and(|id| {
                athlete.athlete_id == 0 || id != athlete.athlete_id
            }) {
                note(&mut athlete, "profile URL athlete ID conflicts with record ID");
                continue;
            }
            if !athlete.profile_urls.contains(&valid) {
                athlete.profile_urls.push(valid);
            }
        }
    }
    athlete.profile_url = match athlete.profile_urls.first() {
        Some(url) => url.clone(),
        None => String::new(),
    };

    let mut source_url = String::new();
    if let Some(source_field) = record.fields.get("source_url") {
        for raw_url in source_field.split(';').map(str::trim).filter(|url| !url.is_empty()) {
            let Some(valid) = validate_source_url(raw_url) else {
                note(&mut athlete, "invalid source URL evidence");
                continue;
            };
            if source_url.is_empty() {
                source_url = valid.clone();
            }
            if !athlete.source_urls.contains(&valid) {
                athlete.source_urls.push(valid);
            }
        }
    }
    if source_url.is_empty() {
        source_url = athlete.profile_url.clone();
    }

    if let Some(results_field) = record.fields.get("result_urls") {
        for raw_url in results_field.split(';').map(str::trim).filter(|url| !url.is_empty()) {
            let Some(valid) = validate_result_url(raw_url) else {
                note(&mut athlete, "invalid result URL evidence");
                continue;
            };
            if !athlete.results.iter().any(|result| result.result_url.as_ref() == Some(&valid)) {
                athlete.results.push(SourceResult {
                    result_id: result_id(&valid),
                    result_url: Some(valid),
                    source_url: source_url.clone(),
                    ..Default::default()
                });
            }
        }
    }

    let ids = record.fields.get("result_ids")
        .filter(|raw| !raw.trim().is_empty())
        .map_or_else(Vec::new, |raw| parsed_ids(raw, &mut athlete));
    if let Some(marks_raw) = record.fields.get("marks") {
        let mut mark_index = 0;
        for raw_mark in marks_raw.split(';').map(str::trim) {
            if raw_mark.is_empty() {
                continue;
            }
            let id = ids.get(mark_index).copied().flatten();
            mark_index += 1;
            if let Some(result) = parse_mark_entry(raw_mark, &source_url, id) {
                athlete.results.push(result);
            } else {
                note(&mut athlete, "invalid mark evidence");
            }
        }
        if ids.len() > mark_index {
            note(&mut athlete, "unpaired result ID evidence");
        }
    } else if !ids.is_empty() {
        note(&mut athlete, "unpaired result ID evidence");
    }

    athlete
}
