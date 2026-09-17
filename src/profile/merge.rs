use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use anyhow::{bail, Result};

use crate::domain::evidence::{
    EvidenceIssue, ProfileEvidence, ResultEvidence, Sport, SportAvailability, TeamEvidence,
};
use crate::domain::facts::{CityName, RegionName, SchoolName};

pub fn merge_profiles(
    mut left: ProfileEvidence,
    right: ProfileEvidence,
) -> Result<ProfileEvidence> {
    if left.athlete_id != right.athlete_id {
        bail!("cannot merge profiles for different athletes");
    }
    if left.name.value != right.name.value {
        left.issues
            .push(conflict("identity_conflict", "profile names conflict"));
    }
    merge_teams(&mut left, right.teams);
    merge_grades(&mut left, right.grades);
    merge_graduation_years(&mut left, right.graduation_years);
    merge_sports(&mut left, right.sports);
    merge_results(&mut left, right.results);
    append_unique(&mut left.issues, right.issues);
    append_unique(&mut left.documents, right.documents);
    Ok(left)
}

#[derive(Default)]
struct TeamObservations<'a> {
    names: HashSet<&'a SchoolName>,
    cities: HashSet<&'a CityName>,
    regions: HashSet<&'a RegionName>,
    levels: HashSet<u8>,
}

fn merge_teams(profile: &mut ProfileEvidence, incoming: Vec<TeamEvidence>) {
    let existing = std::mem::take(&mut profile.teams);
    let observations = existing.iter().fold(
        HashMap::<u64, TeamObservations<'_>>::new(),
        |mut index, value| {
            let observation = index.entry(value.team_id).or_default();
            observation.names.insert(&value.name.value);
            if let Some(location) = value.location.as_ref() {
                let (city, region) = location.value.fields();
                if let Some(city) = city {
                    observation.cities.insert(city);
                }
                if let Some(region) = region {
                    observation.regions.insert(region);
                }
            }
            if let Some(level) = value.level {
                observation.levels.insert(level);
            }
            index
        },
    );
    incoming.iter().for_each(|value| {
        let has_conflict = observations.get(&value.team_id).is_some_and(|known| {
            known.names.len() > 1
                || !known.names.contains(&value.name.value)
                || value.location.as_ref().is_some_and(|location| {
                    let (city, region) = location.value.fields();
                    city.is_some_and(|city| {
                        !known.cities.is_empty()
                            && (known.cities.len() > 1 || !known.cities.contains(city))
                    }) || region.is_some_and(|region| {
                        !known.regions.is_empty()
                            && (known.regions.len() > 1 || !known.regions.contains(region))
                    })
                })
                || value.level.is_some_and(|level| {
                    !known.levels.is_empty()
                        && (known.levels.len() > 1 || !known.levels.contains(&level))
                })
        });
        if has_conflict {
            profile.issues.push(conflict(
                "team_conflict",
                "same team identity has conflicting factual observations",
            ));
        }
    });
    drop(observations);
    profile.teams = existing;
    append_unique(&mut profile.teams, incoming);
}

fn merge_grades(
    profile: &mut ProfileEvidence,
    incoming: Vec<crate::domain::evidence::GradeAtSeason>,
) {
    let existing = std::mem::take(&mut profile.grades);
    let observations = existing.iter().fold(
        HashMap::<(u64, u16), HashSet<u8>>::new(),
        |mut index, value| {
            index
                .entry((value.team_id, value.season))
                .or_default()
                .insert(value.grade);
            index
        },
    );
    incoming.iter().for_each(|value| {
        let has_conflict = observations
            .get(&(value.team_id, value.season))
            .is_some_and(|known| known.len() > 1 || !known.contains(&value.grade));
        if has_conflict {
            profile.issues.push(conflict(
                "grade_conflict",
                "same team-season has conflicting grades",
            ));
        }
    });
    drop(observations);
    profile.grades = existing;
    append_unique(&mut profile.grades, incoming);
}

fn merge_graduation_years(
    profile: &mut ProfileEvidence,
    incoming: Vec<crate::domain::evidence::Observed<crate::domain::facts::GraduationYear>>,
) {
    if !profile.graduation_years.is_empty() && !incoming.is_empty() {
        let incoming_values = incoming
            .iter()
            .map(|value| value.value.get())
            .collect::<HashSet<_>>();
        if profile
            .graduation_years
            .iter()
            .all(|value| !incoming_values.contains(&value.value.get()))
        {
            profile.issues.push(conflict(
                "cohort_conflict",
                "graduation-year witnesses conflict",
            ));
        }
    }
    append_unique(&mut profile.graduation_years, incoming);
}

fn merge_sports(profile: &mut ProfileEvidence, incoming: Vec<SportAvailability>) {
    let existing = std::mem::take(&mut profile.sports);
    let observations = existing.iter().fold(
        HashMap::<Sport, HashSet<&SportAvailability>>::new(),
        |mut index, value| {
            index.entry(sport(value)).or_default().insert(value);
            index
        },
    );
    incoming.iter().for_each(|value| {
        let has_conflict = observations
            .get(&sport(value))
            .is_some_and(|known| known.len() > 1 || !known.contains(&value));
        if has_conflict {
            profile.issues.push(conflict(
                "sport_conflict",
                "same sport availability observations conflict",
            ));
        }
    });
    profile.sports = existing;
    append_unique(&mut profile.sports, incoming);
}

fn merge_results(profile: &mut ProfileEvidence, incoming: Vec<ResultEvidence>) {
    let existing = std::mem::take(&mut profile.results);
    let mut identities = HashSet::new();
    let mut equivalents = HashSet::new();
    existing.iter().for_each(|result| {
        identities.insert((result.result_id, result.sport));
        equivalents.insert(EquivalentResultKey::new(result));
    });
    let mut keep = vec![false; incoming.len()];
    keep.iter_mut()
        .zip(incoming.iter())
        .for_each(|(slot, result)| {
            let key = EquivalentResultKey::new(result);
            if equivalents.contains(&key) {
                return;
            }
            if identities.contains(&(result.result_id, result.sport)) {
                profile.issues.push(conflict(
                    "result_conflict",
                    "same result identity has conflicting observations",
                ));
            }
            identities.insert((result.result_id, result.sport));
            equivalents.insert(key);
            *slot = true;
        });
    drop(equivalents);
    drop(identities);
    profile.results = existing;
    profile.results.extend(
        incoming
            .into_iter()
            .zip(keep)
            .filter_map(|(value, retain)| retain.then_some(value)),
    );
}

#[derive(PartialEq, Eq, Hash)]
struct EquivalentResultKey<'a> {
    result_id: u64,
    sport: Sport,
    event_id: Option<u64>,
    event_name: &'a String,
    event_description: &'a Option<String>,
    event_type: &'a Option<String>,
    mark: &'a String,
    units: &'a Option<String>,
    season: u16,
    team_id: u64,
    meet_id: u64,
    meet_name: &'a Option<String>,
    date: &'a Option<String>,
    wind: &'a Option<String>,
    timing: &'a Option<String>,
    personal_best: &'a crate::domain::evidence::BestClaim,
    season_best: &'a crate::domain::evidence::BestClaim,
    attribution: &'a crate::domain::evidence::ResultAttribution,
    short_code: &'a Option<String>,
    result_url: &'a Option<String>,
}

impl<'a> EquivalentResultKey<'a> {
    fn new(value: &'a ResultEvidence) -> Self {
        Self {
            result_id: value.result_id,
            sport: value.sport,
            event_id: value.event_id,
            event_name: &value.event_name,
            event_description: &value.event_description,
            event_type: &value.event_type,
            mark: &value.mark,
            units: &value.units,
            season: value.season,
            team_id: value.team_id,
            meet_id: value.meet_id,
            meet_name: &value.meet_name,
            date: &value.date,
            wind: &value.wind,
            timing: &value.timing,
            personal_best: &value.personal_best,
            season_best: &value.season_best,
            attribution: &value.attribution,
            short_code: &value.short_code,
            result_url: &value.result_url,
        }
    }
}

fn append_unique<T: Eq + Hash>(target: &mut Vec<T>, incoming: Vec<T>) {
    let existing = std::mem::take(target);
    let mut seen = existing.iter().collect::<HashSet<_>>();
    let mut keep = vec![false; incoming.len()];
    keep.iter_mut()
        .zip(incoming.iter())
        .for_each(|(slot, value)| {
            if seen.insert(value) {
                *slot = true;
            }
        });
    drop(seen);
    *target = existing;
    target.extend(
        incoming
            .into_iter()
            .zip(keep)
            .filter_map(|(value, retain)| retain.then_some(value)),
    );
}

fn conflict(code: &str, message: &str) -> EvidenceIssue {
    EvidenceIssue {
        code: code.to_owned(),
        message: message.to_owned(),
        evidence: None,
    }
}

fn sport(value: &SportAvailability) -> Sport {
    match value {
        SportAvailability::ResultsObserved { sport, .. }
        | SportAvailability::EmptyResponse { sport }
        | SportAvailability::Unavailable { sport } => *sport,
    }
}
