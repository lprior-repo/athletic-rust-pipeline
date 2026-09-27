
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalPerformance, CanonicalSchool, EventKind,
};
use std::collections::{BTreeMap, BTreeSet};

use crate::bests::SharedSelection;

#[derive(Debug, Default, Clone)]
pub(super) struct AthleteTally {
    pub(super) performances: usize,
    pub(super) meets: BTreeSet<String>,
    pub(super) events: BTreeSet<String>,
}

pub(super) fn school_index(schools: &[CanonicalSchool]) -> BTreeMap<String, CanonicalSchool> {
    schools
        .iter()
        .map(|school| (school.id.as_str().to_string(), school.clone()))
        .collect()
}


pub(super) fn kind_index(events: &[CanonicalEvent]) -> BTreeMap<String, EventKind> {
    events
        .iter()
        .map(|event| (event.id.as_str().to_string(), event.kind.clone()))
        .collect()
}


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
            tally.events.insert(kind.stable_key().into_owned());
        }
    }
    tallies
}

pub(super) fn pr_index(prs: &[SharedSelection]) -> BTreeMap<String, Vec<usize>> {
    let mut index: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (position, pr) in prs.iter().enumerate() {
        index
            .entry(pr.athlete_id().as_str().to_string())
            .or_default()
            .push(position);
    }
    index
}
