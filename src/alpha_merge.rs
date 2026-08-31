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
fn approved_profile(athlete_id: u64, raw_url: &str) -> Option<String> {
    let canonical = validate_profile_url(raw_url)?;
    let parsed = Url::parse(&canonical).ok()?;
    let profile_id = parsed.path().strip_prefix("/athlete/")?.parse::<u64>().ok()?;
    (profile_id == athlete_id).then_some(canonical)
}
fn sanitize_profiles(athlete_id: u64, mut athlete: SourceAthlete) -> SourceAthlete {
    let mut profiles = athlete.profile_urls.clone();
    if !athlete.profile_url.is_empty() {
        profiles.push(athlete.profile_url.clone());
    }
    let valid: Vec<String> = profiles
        .iter()
        .filter_map(|url| approved_profile(athlete_id, url))
        .collect();
    if valid.len() != profiles.len() {
        add_note(&mut athlete, "invalid profile URL evidence discarded during merge".to_owned());
    }
    athlete.profile_urls = valid;
    athlete.profile_urls.sort();
    athlete.profile_urls.dedup();
    athlete.profile_url = match athlete.profile_urls.first() {
        Some(url) => url.clone(),
        None => String::new(),
    };
    athlete
}

pub fn merge_athlete(
    map: &mut BTreeMap<u64, SourceAthlete>,
    mut incoming: SourceAthlete,
) -> SourceAthlete {
    if incoming.athlete_id == 0 {
        incoming.profile_urls.clear();
        incoming.profile_url.clear();
        add_note(&mut incoming, "athlete_id missing or zero; cannot trust profile URL".to_owned());
        return incoming;
    }
    let id = incoming.athlete_id;
    incoming = sanitize_profiles(id, incoming);
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
    let existing_cohort_rank = if existing.graduation_year.is_some() { 2 } else { 1 };
    let incoming_cohort_rank = if incoming.graduation_year.is_some() { 2 } else { 1 };
    if incoming_cohort_rank > existing_cohort_rank
        || (existing.cohort_evidence.is_empty() && !incoming.cohort_evidence.is_empty())
    {
        existing.cohort_evidence = incoming.cohort_evidence.clone();
    } else if incoming_cohort_rank == existing_cohort_rank {
        merge_text(
            &mut existing.cohort_evidence,
            &incoming.cohort_evidence,
            "cohort_evidence",
            &mut notes,
        );
    }
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
    let mut existing_profiles = existing.profile_urls.clone();
    if !existing.profile_url.is_empty() {
        existing_profiles.push(existing.profile_url.clone());
    }
    let valid_existing: Vec<String> = existing_profiles
        .iter()
        .filter_map(|url| approved_profile(id, url))
        .collect();
    if valid_existing.len() != existing_profiles.len() {
        notes.push("invalid profile URL evidence discarded during merge".to_owned());
    }
    existing.profile_urls = valid_existing;
    for url in incoming.profile_urls.iter().chain(std::iter::once(&incoming.profile_url)) {
        if url.is_empty() {
            continue;
        }
        let Some(canonical) = approved_profile(id, url) else {
            notes.push("profile URL conflicts with athlete ID".to_owned());
            continue;
        };
        if !existing.profile_urls.contains(&canonical) {
            existing.profile_urls.push(canonical);
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
        existing.profile_url = match existing.profile_urls.first() {
            Some(url) => url.clone(),
            None => String::new(),
        };
    } else if let Some(canonical) = approved_profile(id, &existing.profile_url) {
        existing.profile_url = canonical;
    } else {
        existing.profile_url = String::new();
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
