//! The derivations the recruiting read model is assembled from: index builders over the store's
//! merged rows and the per-athlete tallies the §50 sheet prints.
//!
//! Every function here is pure over rows [`super::dataset::Dataset::load`] already read, and every
//! choice it makes is deterministic, so two runs over one store publish byte-identical cells. The
//! per-school contact facts live next door, with the rules that resolve them: [`super::contact`].

use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    EventKind,
};
use std::collections::{BTreeMap, BTreeSet};

use super::prs::{PrRow, SchoolFacts};

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
