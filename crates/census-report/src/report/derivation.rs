use super::{is_core_evidenced, Scope};
use crate::export::ExportDataset;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool,
};
use census_domain::{JurisdictionBucket, UsJurisdiction};
use std::collections::{HashMap, HashSet};

mod aliases;
mod cohort;
mod population;
pub(super) use aliases::collapse_athletes;
pub(crate) use cohort::candidates as cohort_candidates;

pub(crate) fn in_run_scope(bucket: JurisdictionBucket) -> bool {
    bucket
        .jurisdiction()
        .is_none_or(UsJurisdiction::is_in_census_scope)
}

pub(crate) fn exclude_out_of_scope<T>(
    rows: &mut Vec<T>,
    place: impl Fn(&T) -> JurisdictionBucket,
) -> Vec<T> {
    let (kept, excluded): (Vec<T>, Vec<T>) =
        rows.drain(..).partition(|row| in_run_scope(place(row)));
    *rows = kept;
    excluded
}

pub struct Derivation<'a> {
    dataset: &'a ExportDataset,
    scope: Scope,
    grad_year: Option<i16>,
    school_state: HashMap<&'a str, Option<UsJurisdiction>>,
    schools: Vec<CanonicalSchool>,
    outside_schools: Vec<CanonicalSchool>,
    athletes: Vec<CanonicalAthlete>,
    outside_athletes: Vec<CanonicalAthlete>,
    scoped_athletes: usize,
    meets: Vec<CanonicalMeet>,
    outside_meets: Vec<CanonicalMeet>,
    coaches: Vec<CanonicalCoach>,
    outside_coaches: Vec<CanonicalCoach>,
    coach_observations: Vec<CanonicalCoach>,
    events: Vec<CanonicalEvent>,
    performances: Vec<&'a CanonicalPerformance>,
    athlete_aliases: &'a HashMap<String, String>,
    dropped_rows: usize,
}

impl<'a> Derivation<'a> {
    pub fn of(dataset: &'a ExportDataset, scope: Scope, grad_year: Option<i16>) -> Self {
        let mut derivation = Self::population(dataset, scope, grad_year);
        derivation.retain_geography();
        derivation.retain_evidence_scope();
        derivation.select_cohort();
        derivation
    }

    pub(crate) fn dataset(&self) -> &'a ExportDataset {
        self.dataset
    }

    pub(crate) const fn scope(&self) -> Scope {
        self.scope
    }

    pub(crate) const fn grad_year(&self) -> Option<i16> {
        self.grad_year
    }

    pub(crate) fn school_state(&self) -> &HashMap<&'a str, Option<UsJurisdiction>> {
        &self.school_state
    }

    pub fn schools(&self) -> &[CanonicalSchool] {
        &self.schools
    }

    pub(crate) fn outside_schools(&self) -> &[CanonicalSchool] {
        &self.outside_schools
    }

    pub fn athletes(&self) -> &[CanonicalAthlete] {
        &self.athletes
    }

    pub(crate) fn outside_athletes(&self) -> &[CanonicalAthlete] {
        &self.outside_athletes
    }

    pub(crate) const fn scoped_athletes(&self) -> usize {
        self.scoped_athletes
    }

    pub fn meets(&self) -> &[CanonicalMeet] {
        &self.meets
    }

    pub(crate) fn outside_meets(&self) -> &[CanonicalMeet] {
        &self.outside_meets
    }

    pub fn coaches(&self) -> &[CanonicalCoach] {
        &self.coaches
    }

    pub(crate) fn outside_coaches(&self) -> &[CanonicalCoach] {
        &self.outside_coaches
    }

    pub(crate) fn coach_observations(&self) -> &[CanonicalCoach] {
        &self.coach_observations
    }

    pub(crate) fn events(&self) -> &[CanonicalEvent] {
        &self.events
    }

    pub fn performances(&self) -> &[&'a CanonicalPerformance] {
        &self.performances
    }

    pub(crate) fn athlete_aliases(&self) -> &'a HashMap<String, String> {
        self.athlete_aliases
    }

    pub(crate) const fn dropped_rows(&self) -> usize {
        self.dropped_rows
    }
}

fn cohort_performances<'d>(
    dataset: &'d ExportDataset,
    scope: Scope,
    grad_year: Option<i16>,
    cohort: &HashSet<&str>,
    aliases: &HashMap<String, String>,
) -> Vec<&'d CanonicalPerformance> {
    let known_athletes = known_athletes(dataset, grad_year);
    dataset
        .performances
        .iter()
        .filter(|performance| scope != Scope::Core || is_core_evidenced(*performance))
        .filter(|performance| {
            let subject = performance.athlete.as_str();
            let canonical = aliases.get(subject).map_or(subject, String::as_str);
            cohort.contains(canonical)
                || known_athletes
                    .as_ref()
                    .is_some_and(|known| !known.contains(subject))
        })
        .collect()
}

fn known_athletes(dataset: &ExportDataset, grad_year: Option<i16>) -> Option<HashSet<&str>> {
    grad_year.is_none().then(|| {
        dataset
            .athletes
            .iter()
            .map(|athlete| athlete.id.as_str())
            .collect()
    })
}
