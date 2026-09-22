//! The derivations the recruiting read model is assembled from: index builders over the store's
//! merged rows, the per-athlete tallies the §50 sheet prints, and the per-school contact facts §50 and
//! §53 share.
//!
//! Every function here is pure over rows [`super::dataset::Dataset::load`] already read, and every
//! choice it makes is deterministic (the contact rules are stated on [`contacts`]), so two runs over
//! one store publish byte-identical cells.

use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CoachRole, EventKind, Sport,
};
use std::collections::{BTreeMap, BTreeSet};

use super::prs::{PrRow, SchoolFacts};

/// School-level contact facts the athlete and coach sheets both print.
#[derive(Debug, Default, Clone)]
pub(super) struct SchoolContacts {
    /// Head coach of the track programs: an outdoor head coach when the school has one, else an
    /// indoor one, then by coach name.
    pub(super) head_track: Option<String>,
    pub(super) head_track_email: Option<String>,
    pub(super) head_cross_country: Option<String>,
    pub(super) head_cross_country_email: Option<String>,
    /// The head track coach's professional email, else the head cross-country coach's.
    pub(super) coach_email: Option<String>,
    pub(super) director: Option<String>,
    pub(super) director_email: Option<String>,
}

/// One athlete's stored performance tallies.
#[derive(Debug, Default, Clone)]
pub(super) struct AthleteTally {
    pub(super) performances: usize,
    pub(super) meets: BTreeSet<String>,
    pub(super) events: BTreeSet<String>,
}

/// School id -> school row.
pub(super) fn school_index(schools: &[CanonicalSchool]) -> BTreeMap<String, CanonicalSchool> {
    schools
        .iter()
        .map(|school| (school.id.as_str().to_string(), school.clone()))
        .collect()
}

/// School id -> the facts a PR row prints, in the shape `prs` reduces against.
pub(super) fn school_facts(
    schools: &BTreeMap<String, CanonicalSchool>,
) -> BTreeMap<String, SchoolFacts> {
    schools
        .iter()
        .map(|(id, school)| {
            (
                id.clone(),
                SchoolFacts {
                    name: school.name.clone(),
                    state: school.state.into(),
                },
            )
        })
        .collect()
}

/// Event id -> event kind.
pub(super) fn kind_index(events: &[CanonicalEvent]) -> BTreeMap<String, EventKind> {
    events
        .iter()
        .map(|event| (event.id.as_str().to_string(), event.kind.clone()))
        .collect()
}

/// Meet id -> meet row.
pub(super) fn meet_index(meets: Vec<CanonicalMeet>) -> BTreeMap<String, CanonicalMeet> {
    meets
        .into_iter()
        .map(|meet| (meet.id.as_str().to_string(), meet))
        .collect()
}

/// One tally per published athlete; performances of athletes outside the cohort are not counted.
pub(super) fn tally(
    athletes: &[CanonicalAthlete],
    performances: &[CanonicalPerformance],
    kinds: &BTreeMap<String, EventKind>,
) -> BTreeMap<String, AthleteTally> {
    let cohort: BTreeSet<&str> = athletes.iter().map(|athlete| athlete.id.as_str()).collect();
    let mut tallies: BTreeMap<String, AthleteTally> = athletes
        .iter()
        .map(|athlete| (athlete.id.as_str().to_string(), AthleteTally::default()))
        .collect();
    for performance in performances {
        if !cohort.contains(performance.athlete.as_str()) {
            continue;
        }
        let Some(tally) = tallies.get_mut(performance.athlete.as_str()) else {
            continue;
        };
        tally.performances = tally.performances.saturating_add(1);
        tally.meets.insert(performance.meet.as_str().to_string());
        if let Some(kind) = kinds.get(performance.event.as_str()) {
            tally.events.insert(format!("{kind:?}"));
        }
    }
    tallies
}

/// Athlete id -> the positions of that athlete's PR rows, so the athlete sheet never rescans the PR
/// list per row.
pub(super) fn pr_index(prs: &[PrRow]) -> BTreeMap<String, Vec<usize>> {
    let mut index: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (position, pr) in prs.iter().enumerate() {
        index
            .entry(pr.athlete_id.clone())
            .or_default()
            .push(position);
    }
    index
}

/// School id -> contact facts. Head coaches are chosen deterministically: outdoor track before indoor
/// track, then by coach name, and the school's first-named athletic director by the same rule.
pub(super) fn contacts(coaches: &[CanonicalCoach]) -> BTreeMap<String, SchoolContacts> {
    let mut heads: BTreeMap<&str, Vec<&CanonicalCoach>> = BTreeMap::new();
    let mut directors: BTreeMap<&str, Vec<&CanonicalCoach>> = BTreeMap::new();
    let mut index: BTreeMap<String, SchoolContacts> = BTreeMap::new();
    for coach in coaches {
        match coach.role {
            CoachRole::HeadCoach if coach.sport.is_some() => {
                heads.entry(coach.school.as_str()).or_default().push(coach);
            }
            CoachRole::AthleticDirector => {
                directors
                    .entry(coach.school.as_str())
                    .or_default()
                    .push(coach);
            }
            CoachRole::HeadCoach | CoachRole::AssistantCoach | CoachRole::Unknown => {}
        }
    }
    for (school, mut candidates) in heads {
        candidates.sort_by_key(|coach| (track_rank(coach.sport), coach.name.clone()));
        let entry = index.entry(school.to_string()).or_default();
        let track = candidates
            .iter()
            .find(|coach| is_track_sport(coach.sport))
            .copied();
        let cross_country = candidates
            .iter()
            .find(|coach| coach.sport == Some(Sport::CrossCountry))
            .copied();
        entry.head_track = track.map(|coach| coach.name.clone());
        entry.head_track_email = track.and_then(|coach| coach.professional_email.clone());
        entry.head_cross_country = cross_country.map(|coach| coach.name.clone());
        entry.head_cross_country_email =
            cross_country.and_then(|coach| coach.professional_email.clone());
        entry.coach_email = entry
            .head_track_email
            .clone()
            .or_else(|| entry.head_cross_country_email.clone());
    }
    for (school, mut candidates) in directors {
        candidates.sort_by_key(|coach| coach.name.clone());
        let entry = index.entry(school.to_string()).or_default();
        let director = candidates.first().copied();
        entry.director = director.map(|coach| coach.name.clone());
        entry.director_email = director.and_then(|coach| coach.professional_email.clone());
    }
    index
}

/// Head-coach sort rank: outdoor track first, then indoor track, then cross country.
fn track_rank(sport: Option<Sport>) -> u8 {
    match sport {
        Some(Sport::OutdoorTrack) => 0,
        Some(Sport::IndoorTrack) => 1,
        _ => 2,
    }
}

fn is_track_sport(sport: Option<Sport>) -> bool {
    matches!(sport, Some(Sport::OutdoorTrack) | Some(Sport::IndoorTrack))
}
