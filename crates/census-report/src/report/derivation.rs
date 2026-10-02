use super::coverage::{in_cohort, jurisdiction_of, school_state_index};
use super::{is_core_evidenced, retain_core, Scope};
use crate::export::ExportDataset;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, Gender,
};
use census_domain::{JurisdictionBucket, UsJurisdiction};
use std::collections::{BTreeMap, HashMap, HashSet};

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
        let school_state = school_state_index(dataset.schools.values());
        let mut schools: Vec<CanonicalSchool> = dataset.schools.values().cloned().collect();
        let outside_schools = exclude_out_of_scope(&mut schools, |school| school.state.into());
        let mut meets: Vec<CanonicalMeet> = dataset.meets.clone();
        let outside_meets = exclude_out_of_scope(&mut meets, |meet| meet.state.into());
        let athlete_aliases = &dataset.canonical_aliases;
        let mut athletes: Vec<CanonicalAthlete> =
            collapse_athletes(&dataset.athletes, athlete_aliases);
        let outside_athletes = exclude_out_of_scope(&mut athletes, |athlete| {
            jurisdiction_of(&school_state, athlete.school.as_str())
        });
        let mut coaches: Vec<CanonicalCoach> = dataset.coaches.clone();
        let outside_coaches = exclude_out_of_scope(&mut coaches, |coach| {
            jurisdiction_of(&school_state, coach.school.as_str())
        });
        let mut coach_observations: Vec<CanonicalCoach> = dataset.coach_observations.clone();
        coach_observations
            .retain(|coach| in_run_scope(jurisdiction_of(&school_state, coach.school.as_str())));
        let mut events = dataset.events.clone();
        let dropped_rows = if scope == Scope::Core {
            retain_core(&mut events);
            retain_core(&mut athletes).saturating_add(retain_core(&mut meets))
        } else {
            0
        };
        let scoped_athletes = athletes.len();
        athletes.retain(|athlete| in_cohort(athlete, grad_year));
        let cohort: HashSet<&str> = athletes.iter().map(|athlete| athlete.id.as_str()).collect();
        let performances = cohort_performances(dataset, scope, &cohort, athlete_aliases);
        Self {
            dataset,
            scope,
            grad_year,
            school_state,
            schools,
            outside_schools,
            athletes,
            outside_athletes,
            scoped_athletes,
            meets,
            outside_meets,
            coaches,
            outside_coaches,
            coach_observations,
            events,
            performances,
            athlete_aliases,
            dropped_rows,
        }
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
    cohort: &HashSet<&str>,
    aliases: &HashMap<String, String>,
) -> Vec<&'d CanonicalPerformance> {
    let known_athletes: HashSet<&str> = dataset
        .athletes
        .iter()
        .map(|athlete| athlete.id.as_str())
        .collect();
    dataset
        .performances
        .iter()
        .filter(|performance| scope != Scope::Core || is_core_evidenced(*performance))
        .filter(|performance| {
            let subject = performance.athlete.as_str();
            let canonical = aliases.get(subject).map_or(subject, String::as_str);
            cohort.contains(canonical) || !known_athletes.contains(subject)
        })
        .collect()
}

pub(super) fn collapse_athletes(
    athletes: &[CanonicalAthlete],
    aliases: &HashMap<String, String>,
) -> Vec<CanonicalAthlete> {
    if aliases.is_empty() {
        return athletes.to_vec();
    }
    let groups = athletes.iter().fold(
        BTreeMap::<&str, Vec<&CanonicalAthlete>>::new(),
        |mut groups, athlete| {
            let canonical = aliases
                .get(athlete.id.as_str())
                .map_or(athlete.id.as_str(), String::as_str);
            groups.entry(canonical).or_default().push(athlete);
            groups
        },
    );
    let capacity = groups.len();
    groups.into_iter().fold(
        Vec::with_capacity(capacity),
        |mut collapsed, (canonical, members)| {
            match members
                .iter()
                .find(|member| member.id.as_str() == canonical)
            {
                Some(representative) => {
                    collapsed.push(union_accepted_members(representative, &members));
                }
                None => collapsed.extend(members.into_iter().cloned()),
            }
            collapsed
        },
    )
}

fn union_accepted_members(
    representative: &CanonicalAthlete,
    members: &[&CanonicalAthlete],
) -> CanonicalAthlete {
    let mut row = representative.clone();
    let known_gender = members
        .iter()
        .map(|member| member.gender)
        .filter(|gender| *gender != Gender::Unknown);
    if row.gender == Gender::Unknown {
        let mut genders = known_gender;
        if let Some(first) = genders.next() {
            if genders.all(|gender| gender == first) {
                row.gender = first;
            }
        }
    }
    members
        .iter()
        .filter(|member| member.id != representative.id)
        .for_each(|member| {
            union_values(&mut row.known_names, &member.known_names);
            union_values(
                &mut row.known_names,
                std::slice::from_ref(&member.canonical_name),
            );
            union_values(&mut row.sports, &member.sports);
            union_values(&mut row.public_profile_urls, &member.public_profile_urls);
            union_values(&mut row.observed_grades, &member.observed_grades);
            union_values(&mut row.evidence, &member.evidence);
            union_values(&mut row.retained_conflicts, &member.retained_conflicts);
            member.identities().for_each(|identity| {
                if let Some(url) = &identity.url {
                    union_values(&mut row.public_profile_urls, std::slice::from_ref(url));
                }
                row.add_identity(identity.clone());
            });
        });
    row
}

fn union_values<T: PartialEq + Clone>(target: &mut Vec<T>, values: &[T]) {
    values.iter().for_each(|value| {
        if !target.contains(value) {
            target.push(value.clone());
        }
    });
}
