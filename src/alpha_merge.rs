/// Athlete merging, deduplication, and ranking-record conversion.
use crate::alpha_model::{RankingRecord, SourceAthlete, SourceResult};
use crate::alpha_url::validate_profile_url;
use std::collections::BTreeMap;
use url::Url;

#[allow(dead_code)]
pub fn from_ranking_record(
    record: &RankingRecord,
    profile_url: &str,
) -> Result<SourceResult, String> {
    let profile = validate_profile_url(profile_url)
        .ok_or_else(|| "ranking profile URL is not an approved Athletic.net profile".to_owned())?;
    let profile_id = Url::parse(&profile)
        .ok()
        .and_then(|url| url.path().strip_prefix("/athlete/").map(str::to_owned))
        .and_then(|id| id.parse::<u64>().ok());
    if profile_id != Some(record.athlete_id) {
        return Err("ranking profile URL athlete ID does not match record ID".to_owned());
    }
    Ok(SourceResult {
        result_id: record.result_id,
        event: record.event_short.clone(),
        mark: record.measure.clone(),
        season: record.season_id.to_string(),
        date: record.result_date.clone(),
        meet_name: record.meet_name.clone(),
        wind: record.wind.clone(),
        source_url: profile,
        result_url: None,
    })
}

#[allow(dead_code)]
pub fn from_model_source_result(result: &SourceResult) -> SourceResult {
    result.clone()
}


fn add_note(athlete: &mut SourceAthlete, message: String) {
    if !athlete.exception_notes.contains(&message) {
        athlete.exception_notes.push(message);
    }
}

fn merge_text(
    existing: &mut String,
    incoming: &str,
    field: &str,
    notes: &mut Vec<String>,
) {
    if existing.is_empty() {
        *existing = incoming.to_owned();
    } else if !incoming.is_empty() && existing != incoming {
        notes.push(format!("{field} conflict"));
    }
}

fn merge_optional(
    existing: &mut Option<i32>,
    incoming: Option<i32>,
    field: &str,
    notes: &mut Vec<String>,
) {
    match (*existing, incoming) {
        (None, Some(value)) => *existing = Some(value),
        (Some(left), Some(right)) if left != right => notes.push(format!("{field} conflict")),
        _ => {}
    }
}

fn merge_result(existing: &mut Vec<SourceResult>, incoming: &SourceResult) {
    let duplicate = existing.iter().any(|candidate| {
        candidate.result_id == incoming.result_id
            && candidate.event == incoming.event
            && candidate.mark == incoming.mark
            && candidate.season == incoming.season
            && candidate.date == incoming.date
            && candidate.meet_name == incoming.meet_name
            && candidate.wind == incoming.wind
            && candidate.source_url == incoming.source_url
            && candidate.result_url == incoming.result_url
    });
    if !duplicate {
        existing.push(incoming.clone());
    }
}

pub fn merge_athlete(
    map: &mut BTreeMap<u64, SourceAthlete>,
    mut incoming: SourceAthlete,
) -> SourceAthlete {
    if incoming.athlete_id == 0 {
        add_note(&mut incoming, "athlete_id missing or zero; cannot deduplicate".to_owned());
        return incoming;
    }
    let id = incoming.athlete_id;
    let Some(existing) = map.get_mut(&id) else {
        map.insert(id, incoming.clone());
        return incoming;
    };
    let mut notes = Vec::new();
    merge_text(&mut existing.athlete_name, &incoming.athlete_name, "athlete_name", &mut notes);
    merge_text(&mut existing.first_name, &incoming.first_name, "first_name", &mut notes);
    merge_text(&mut existing.last_name, &incoming.last_name, "last_name", &mut notes);
    merge_text(&mut existing.school, &incoming.school, "school", &mut notes);
    merge_text(&mut existing.team_name, &incoming.team_name, "team_name", &mut notes);
    merge_text(&mut existing.state, &incoming.state, "state", &mut notes);
    merge_text(&mut existing.city, &incoming.city, "city", &mut notes);
    merge_text(&mut existing.gender, &incoming.gender, "gender", &mut notes);
    merge_text(&mut existing.sport, &incoming.sport, "sport", &mut notes);
    merge_text(
        &mut existing.cohort_evidence,
        &incoming.cohort_evidence,
        "cohort_evidence",
        &mut notes,
    );
    if existing.grade_id == 0 {
        existing.grade_id = incoming.grade_id;
    } else if incoming.grade_id != 0 && existing.grade_id != incoming.grade_id {
        notes.push("grade_id conflict".to_owned());
    }
    merge_optional(
        &mut existing.graduation_year,
        incoming.graduation_year,
        "graduation_year",
        &mut notes,
    );
    for url in &incoming.profile_urls {
        if !existing.profile_urls.contains(url) {
            existing.profile_urls.push(url.clone());
        }
    }
    for url in &incoming.source_urls {
        if !existing.source_urls.contains(url) {
            existing.source_urls.push(url.clone());
        }
    }
    for result in &incoming.results {
        merge_result(&mut existing.results, result);
    }
    for note in notes.into_iter().chain(incoming.exception_notes) {
        add_note(existing, note);
    }
    if existing.profile_url.is_empty() {
        existing.profile_url = incoming.profile_url;
    }
    existing.clone()
}

#[allow(dead_code)]
pub fn dedup_athletes(athletes: Vec<SourceAthlete>) -> (Vec<SourceAthlete>, Vec<SourceAthlete>) {
    let mut keyed = BTreeMap::new();
    let mut exception_only = Vec::new();
    for athlete in athletes {
        let merged = merge_athlete(&mut keyed, athlete);
        if merged.athlete_id == 0 {
            exception_only.push(merged);
        }
    }
    (keyed.into_values().collect(), exception_only)
}
