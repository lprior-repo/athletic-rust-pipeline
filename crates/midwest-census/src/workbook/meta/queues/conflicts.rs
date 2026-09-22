//! The conflict families: rows the merge kept separate and left to a human to reconcile.
//!
//! Three families, each one a *stored* disagreement rather than a heuristic dislike. Duplicate school
//! names are the same `(state, normalized name)` key `report` counts as `duplicate_school_names`; the
//! athlete family is the same `(school, name, cohort)` key the identity consolidation groups by, which
//! only ever holds more than one row when the two ids differ in the gender component; the cohort
//! family is every athlete whose own grade observations disagree about the graduating class — in both
//! directions, two implied classes or one implied class other than the canonical one.

use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalSchool, MEET_STATE_UNRESOLVED,
};
use std::collections::{BTreeMap, HashMap};

use super::super::{school_of, subject_of, Family, QueueRow, StoreRows};
use super::{class_of_2027, queue_row, ATHLETE_IDENTITY, COHORT_EVIDENCE, SCHOOL_IDENTITY};

/// Athletes whose own grade observations disagree about the graduating class.
pub(super) fn cohort_evidence(rows: &StoreRows, names: &HashMap<&str, &str>) -> Family {
    let mut family = Family::new(COHORT_EVIDENCE);
    for athlete in class_of_2027(&rows.athletes) {
        let Some(detail) = cohort_conflict(athlete) else {
            continue;
        };
        let subject = subject_of(
            &athlete.canonical_name,
            school_of(names, athlete.school.as_str()),
        );
        family.push(queue_row(athlete.id.as_str(), subject, detail));
    }
    family
}

/// Why an athlete's cohort evidence is unresolved, or `None` when every observation agrees.
///
/// Both directions are conflicts: observations that imply two different classes, and a single
/// observation that implies a class other than the canonical `grad_year` the row carries.
fn cohort_conflict(athlete: &CanonicalAthlete) -> Option<String> {
    let mut implied: Vec<String> = Vec::new();
    let mut observations: Vec<String> = Vec::new();
    for observation in &athlete.observed_grades {
        let year = observation.grad_year().to_string();
        if !implied.contains(&year) {
            implied.push(year);
        }
        observations.push(format!(
            "grade {} in {} from {}",
            observation.grade,
            observation.school_year.short(),
            observation.source.id
        ));
    }
    let canonical = athlete.grad_year.to_string();
    let disagrees = implied.len() > 1 || implied.first().is_some_and(|year| *year != canonical);
    disagrees.then(|| {
        format!(
            "canonical grad year {canonical}, observations imply {}: {}",
            implied.join("/"),
            observations.join("; ")
        )
    })
}

/// Athletes the merge kept twice for one `(school, name, cohort)` key: the two ids differ only in
/// the gender component, so which row is the athlete is unresolved.
pub(super) fn athlete_identity(rows: &StoreRows, names: &HashMap<&str, &str>) -> Family {
    let mut family = Family::new(ATHLETE_IDENTITY);
    let mut groups: BTreeMap<(String, String, i16), Vec<&CanonicalAthlete>> = BTreeMap::new();
    for athlete in class_of_2027(&rows.athletes) {
        let key = (
            athlete.school.as_str().to_string(),
            normalize_name(&athlete.canonical_name),
            athlete.grad_year.get(),
        );
        groups.entry(key).or_default().push(athlete);
    }
    for ((_, _, _), group) in groups.iter().filter(|(_, group)| group.len() > 1) {
        let ids = group
            .iter()
            .map(|athlete| athlete.id.as_str())
            .collect::<Vec<&str>>()
            .join(", ");
        let rows: Vec<QueueRow> = group
            .iter()
            .map(|athlete| {
                let subject = subject_of(
                    &athlete.canonical_name,
                    school_of(names, athlete.school.as_str()),
                );
                queue_row(
                    athlete.id.as_str(),
                    subject,
                    format!("same school, name and cohort as every id here: {ids}"),
                )
            })
            .collect();
        family.group(rows);
    }
    family
}

/// Schools that share one `(state, normalized name)` key: the merge kept both rows, so which one is
/// the real school is unresolved. This is the same key `report` counts as `duplicate_school_names`.
pub(super) fn school_identity(schools: &[CanonicalSchool]) -> Family {
    let mut family = Family::new(SCHOOL_IDENTITY);
    let mut groups: BTreeMap<(String, String), Vec<&CanonicalSchool>> = BTreeMap::new();
    for school in schools {
        let state = school
            .state
            .map_or(String::new(), |state| state.code().to_string());
        groups
            .entry((state, school.normalized_name.clone()))
            .or_default()
            .push(school);
    }
    for ((state, normalized), group) in groups.iter().filter(|(_, group)| group.len() > 1) {
        let state = if state.is_empty() {
            MEET_STATE_UNRESOLVED
        } else {
            state.as_str()
        };
        let ids = group
            .iter()
            .map(|school| school.id.as_str())
            .collect::<Vec<&str>>()
            .join(", ");
        family.group(school_identity_rows(state, normalized, &ids, group));
    }
    family
}

/// One row per school inside a name-collision group, naming every id the collided key covers.
fn school_identity_rows(
    state: &str,
    normalized: &str,
    ids: &str,
    group: &[&CanonicalSchool],
) -> Vec<QueueRow> {
    group
        .iter()
        .map(|school| {
            queue_row(
                school.id.as_str(),
                format!("{} ({state})", school.name),
                format!("normalized name '{normalized}' is shared by ids {ids}"),
            )
        })
        .collect()
}
