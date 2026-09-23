//! Canonical mapping: the accumulated entities, the school/meet resolution and the performance
//! store, plus the published-flag readers they apply to a row.

use super::parse::Bio;
use crate::school_index::SchoolIndex;
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CompetitionLevel, EventId, EventKind, Evidence, Gender, Grade,
    Mark, ObservedGrade, SchoolId, SchoolYear, SourceEventLabel, SourceIdentity, SourceNamespace,
    SourceRef, Sport, TeamId, TimingMethod,
};
use census_domain::UsJurisdiction;
use std::collections::HashMap;

/// Profile URL for an athlete id, in the form Athletic.net itself links to.
pub(super) fn profile_url(athlete_id: u64) -> String {
    format!("https://www.athletic.net/athlete/{athlete_id}/track-and-field")
}

#[derive(Debug, Default)]
pub(super) struct Stats {
    pub(super) athletes_seen: u64,
    pub(super) athletes_absorbed: u64,
    pub(super) athletes_without_grade: u64,
    pub(super) athletes_without_school: u64,
    pub(super) gender_unknown: u64,
    pub(super) rows_seen: u64,
    pub(super) rows_absorbed: u64,
    pub(super) rows_no_mark: u64,
    pub(super) rows_no_season: u64,
    pub(super) rows_unknown_season: HashMap<String, u64>,
    pub(super) rows_no_event: u64,
    pub(super) rows_unknown_school: u64,
    pub(super) rows_unknown_meet: u64,
    pub(super) rows_without_state: u64,
    pub(super) schools_minted: u64,
    pub(super) schools_resolved: u64,
    pub(super) fetches_failed: u64,
}

#[derive(Default)]
pub(super) struct Accumulator {
    pub(super) schools: HashMap<String, CanonicalSchool>,
    pub(super) meets: HashMap<String, CanonicalMeet>,
    pub(super) teams: HashMap<String, CanonicalTeam>,
    pub(super) athletes: HashMap<String, CanonicalAthlete>,
    pub(super) events: HashMap<String, CanonicalEvent>,
    pub(super) performances: HashMap<String, CanonicalPerformance>,
}

/// The grade observed in a given school year, when the payload publishes one.
pub(super) fn grade_in(observed: &[ObservedGrade], school_year: SchoolYear) -> Option<Grade> {
    observed
        .iter()
        .find(|observation| observation.school_year == school_year)
        .map(|observation| observation.grade)
}

/// Resolve or mint the school a payload row names, memoized per run so a school is counted once
/// rather than once per row that names it.
///
/// Returns `None` when the state is unknown or the payload publishes no name for the school id:
/// school identity keys on state + name, so minting without one would merge same-named schools in
/// different states.
#[allow(clippy::too_many_arguments)]
pub(super) fn school_for(
    school_id: &str,
    state: Option<UsJurisdiction>,
    school_names: &HashMap<String, &str>,
    index: &SchoolIndex,
    resolved: &mut HashMap<String, SchoolId>,
    source: &SourceRef,
    observed_on: &str,
    stats: &mut Stats,
    accumulated: &mut Accumulator,
) -> Option<SchoolId> {
    let state = state?;
    let key = format!("{state}:{school_id}");
    if let Some(id) = resolved.get(&key) {
        return Some(id.clone());
    }
    let Some(name) = school_names.get(school_id).map(|name| name.to_string()) else {
        stats.rows_unknown_school = stats.rows_unknown_school.saturating_add(1);
        return None;
    };
    if let Some((id, _)) = index.resolve(state, &name) {
        stats.schools_resolved = stats.schools_resolved.saturating_add(1);
        resolved.insert(key, id.clone());
        return Some(id);
    }
    let (mut school, id) = CanonicalSchool::new(state, name.clone(), name.to_lowercase());
    school.source_identities.push(SourceIdentity {
        namespace: SourceNamespace::AthleticNet {
            kind: "school".to_string(),
        },
        id: school_id.to_string(),
        url: None,
    });
    school
        .evidence
        .push(Evidence::parsed(source.clone(), observed_on));
    stats.schools_minted = stats.schools_minted.saturating_add(1);
    let id = accumulated
        .schools
        .entry(id.as_str().to_string())
        .or_insert(school)
        .id
        .clone();
    resolved.insert(key, id.clone());
    Some(id)
}

/// Resolve or mint the meet a payload row names.
pub(super) fn meet_for(
    meet_id: Option<i64>,
    bio: &Bio,
    state: Option<UsJurisdiction>,
    source: &SourceRef,
    observed_on: &str,
    accumulated: &mut Accumulator,
) -> Option<CanonicalMeet> {
    let state = state?;
    let meet_id = meet_id?;
    let published = bio.meets.get(&meet_id.to_string())?;
    let date = published.date()?.to_string();
    let meet = accumulated
        .meets
        .entry(format!("{state}:{meet_id}"))
        .or_insert_with(|| {
            let mut meet = CanonicalMeet::new(
                Some(state),
                published.name.clone(),
                date.clone(),
                CompetitionLevel::Unknown,
            );
            meet.source_identities.push(SourceIdentity {
                namespace: SourceNamespace::AthleticNet {
                    kind: "meet".to_string(),
                },
                id: meet_id.to_string(),
                url: None,
            });
            meet.evidence
                .push(Evidence::parsed(source.clone(), observed_on));
            meet
        });
    Some(meet.clone())
}

/// Everything one published result row contributes to a canonical performance.
pub(super) struct PerformanceInput<'a> {
    pub(super) athlete: &'a AthleteId,
    pub(super) school: &'a SchoolId,
    pub(super) meet: &'a CanonicalMeet,
    pub(super) kind: &'a EventKind,
    pub(super) sport: Sport,
    pub(super) gender: Gender,
    pub(super) school_year: SchoolYear,
    pub(super) grade: Option<Grade>,
    pub(super) date: String,
    pub(super) mark: Mark,
    pub(super) wind_mps: Option<f64>,
    pub(super) place: Option<&'a str>,
    pub(super) round: Option<String>,
    pub(super) timing: Option<TimingMethod>,
    pub(super) division: Option<String>,
    pub(super) source_key: String,
    pub(super) label: Option<&'a str>,
}

/// Store one canonical performance, minting its team and event on the platform's keys.
#[allow(clippy::too_many_arguments)]
pub(super) fn store_performance(
    accumulated: &mut Accumulator,
    source: &SourceRef,
    observed_on: &str,
    input: PerformanceInput<'_>,
) {
    let team_id = ensure_team(accumulated, source, observed_on, &input);
    let event = ensure_event(accumulated, source, observed_on, &input);

    let performance_id = CanonicalPerformance::mint(
        input.athlete,
        &input.meet.id,
        input.kind,
        &input.date,
        &input.source_key,
    );
    accumulated
        .performances
        .entry(performance_id.as_str().to_string())
        .or_insert_with(|| CanonicalPerformance {
            id: performance_id,
            athlete: input.athlete.clone(),
            team: team_id,
            event,
            meet: input.meet.id.clone(),
            date: input.date,
            mark: input.mark,
            wind_mps: input.wind_mps,
            place: input.place.and_then(|place| place.trim().parse().ok()),
            heat: None,
            round: input.round,
            timing: input.timing,
            observed_grade: input.grade,
            evidence: vec![Evidence::parsed(source.clone(), observed_on)],
            source_key: input.source_key,
            retained_conflicts: Vec::new(),
        });
}

/// The team a performance belongs to, minted on first sight.
pub(super) fn ensure_team(
    accumulated: &mut Accumulator,
    source: &SourceRef,
    observed_on: &str,
    input: &PerformanceInput<'_>,
) -> TeamId {
    let team_id = CanonicalTeam::mint(input.school, input.sport, input.gender, input.school_year);
    if !accumulated.teams.contains_key(team_id.as_str()) {
        accumulated.teams.insert(
            team_id.as_str().to_string(),
            CanonicalTeam {
                id: team_id.clone(),
                school: input.school.clone(),
                sport: input.sport,
                gender: input.gender,
                school_year: input.school_year,
                level: Some("high_school".to_string()),
                source_identities: Vec::new(),
                evidence: vec![Evidence::parsed(source.clone(), observed_on)],
                retained_conflicts: Vec::new(),
            },
        );
    }
    team_id
}

/// The event a performance belongs to, minted on first sight.
fn ensure_event(
    accumulated: &mut Accumulator,
    source: &SourceRef,
    observed_on: &str,
    input: &PerformanceInput<'_>,
) -> EventId {
    accumulated
        .events
        .entry(format!(
            "{}:{:?}:{:?}:{}",
            input.meet.id.as_str(),
            input.kind,
            input.gender,
            input.division.clone().unwrap_or_default()
        ))
        .or_insert_with(|| {
            let mut event = CanonicalEvent::new(
                &input.meet.id,
                input.kind.clone(),
                input.gender,
                input.division.as_deref(),
                input.round.as_deref(),
            );
            if let Some(label) = input.label {
                event.source_labels.push(SourceEventLabel {
                    source: source.clone(),
                    label: label.to_string(),
                });
            }
            event
                .evidence
                .push(Evidence::parsed(source.clone(), observed_on));
            event
        })
        .id
        .clone()
}
