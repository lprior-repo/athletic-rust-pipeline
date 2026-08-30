/// Athlete merging, deduplication, and RankingRecord conversion.
use crate::alpha_model::{RankingRecord, SourceResult as ModelSourceResult};
use crate::alpha_normalize::{ResultRecord, SourceAthlete};
use std::collections::BTreeMap;

/// Convert a RankingRecord into a ResultRecord with full fields preserved.
pub fn from_ranking_record(rec: &RankingRecord, _profile_url: &str) -> ResultRecord {
    ResultRecord {
        result_id: rec.result_id.unwrap_or(0),
        event: rec.event_short.clone(),
        mark: rec.measure.clone(),
        season: rec.season_id.to_string(),
        date: rec.result_date.clone(),
        meet_name: rec.meet_name.clone(),
        wind: rec.wind.clone(),
        source_url: format!("https://athletic.net/athlete/{}", rec.athlete_id),
        result_url: if rec.result_id.is_some() {
            format!("https://athletic.net/result/{}", rec.result_id.unwrap_or(0))
        } else {
            String::new()
        },
    }
}

/// Convert a model SourceResult into a ResultRecord.
pub fn from_model_source_result(sr: &ModelSourceResult, _profile_url: &str) -> ResultRecord {
    ResultRecord {
        result_id: sr.result_id,
        event: sr.event_short.clone(),
        mark: sr.measure.clone(),
        season: sr.season_id.to_string(),
        date: sr.result_date.clone(),
        meet_name: String::new(),
        wind: None,
        result_url: if sr.result_id > 0 {
            format!("https://athletic.net/result/{}", sr.result_id)
        } else {
            String::new()
        },
        ..Default::default()
    }
}

/// Dedup athletes by athlete_id, merging duplicates instead of discarding.
pub fn dedup_athletes(athletes: Vec<SourceAthlete>) -> Vec<SourceAthlete> {
    let mut map: BTreeMap<u64, SourceAthlete> = BTreeMap::new();
    for a in athletes {
        merge_athlete(&mut map, a);
    }
    map.into_values().collect()
}

/// Merge a new SourceAthlete into an existing map by athlete_id.
///
/// Merge rules:
/// - Identity: fill empty first_name/last_name/school/state/city from new
/// - Conflicts: if both non-empty and different → exception note
/// - URLs: retain all distinct profile/source/result URLs
/// - Results: dedup by full event+mark+date+meet+source+result_id+result_url identity
pub fn merge_athlete(map: &mut BTreeMap<u64, SourceAthlete>, new: SourceAthlete) -> u64 {
    let id = new.athlete_id;
    if id == 0 {
        // Missing/zero ID → exception-only record under key 0
        let note = "athlete_id missing or zero; cannot deduplicate".to_owned();
        let mut exn = SourceAthlete {
            athlete_id: 0,
            exception_notes: vec![note],
            ..new
        };
        let existing = map.entry(0).or_default();
        for n in exn.exception_notes.drain(..) {
            if !existing.exception_notes.contains(&n) {
                existing.exception_notes.push(n);
            }
        }
        return 0;
    }

    match map.get_mut(&id) {
        Some(existing) => {
            // Merge identity: fill empty, record conflicts for non-empty mismatches
            if existing.first_name.is_empty() && !new.first_name.is_empty() {
                existing.first_name = new.first_name;
            } else if !existing.first_name.is_empty()
                && !new.first_name.is_empty()
                && existing.first_name != new.first_name
            {
                existing.exception_notes.push(format!(
                    "first_name conflict: '{}' vs '{}'",
                    existing.first_name, new.first_name
                ));
            }

            if existing.last_name.is_empty() && !new.last_name.is_empty() {
                existing.last_name = new.last_name;
            } else if !existing.last_name.is_empty()
                && !new.last_name.is_empty()
                && existing.last_name != new.last_name
            {
                existing.exception_notes.push(format!(
                    "last_name conflict: '{}' vs '{}'",
                    existing.last_name, new.last_name
                ));
            }

            if existing.school.is_empty() && !new.school.is_empty() {
                existing.school = new.school;
            } else if !existing.school.is_empty()
                && !new.school.is_empty()
                && existing.school != new.school
            {
                existing.exception_notes.push(format!(
                    "school conflict: '{}' vs '{}'",
                    existing.school, new.school
                ));
            }

            if existing.state.is_empty() && !new.state.is_empty() {
                existing.state = new.state;
            } else if !existing.state.is_empty()
                && !new.state.is_empty()
                && existing.state != new.state
            {
                existing.exception_notes.push(format!(
                    "state conflict: '{}' vs '{}'",
                    existing.state, new.state
                ));
            }

            if existing.city.is_empty() && !new.city.is_empty() {
                existing.city = new.city;
            } else if !existing.city.is_empty()
                && !new.city.is_empty()
                && existing.city != new.city
            {
                existing.exception_notes.push(format!(
                    "city conflict: '{}' vs '{}'",
                    existing.city, new.city
                ));
            }

            // Merge profile URLs
            for url in new.profile_urls {
                if !existing.profile_urls.contains(&url) {
                    existing.profile_urls.push(url);
                }
            }

            // Merge source URLs
            for url in new.source_urls {
                if !existing.source_urls.contains(&url) {
                    existing.source_urls.push(url);
                }
            }

            // Merge results: dedup by full identity
            for nr in new.results {
                let is_dup = existing.results.iter().any(|r| {
                    // Full identity match
                    nr.result_id == r.result_id
                        && nr.event == r.event
                        && nr.mark == r.mark
                        && nr.date == r.date
                        && nr.meet_name == r.meet_name
                        && nr.source_url == r.source_url
                        && nr.result_url == r.result_url
                });
                if !is_dup {
                    existing.results.push(nr);
                }
            }

            // Merge exception notes
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
