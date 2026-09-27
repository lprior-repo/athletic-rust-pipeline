
use census_domain::model::{CanonicalAthlete, CanonicalSchool, MEET_STATE_UNRESOLVED};
use std::collections::{BTreeMap, HashMap};

use crate::workbook::recruiting::{disagreements, Disagreement};
use census_review::athlete_flags::{key as athlete_key, IdentityKey};

use super::super::{school_of, subject_of, Family, QueueRow, StoreRows};
use super::{
    class_of_2027, queue_row, ATHLETE_IDENTITY, COHORT_EVIDENCE, CONTACT_CONFLICT, SCHOOL_IDENTITY,
};

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

pub(super) fn contact_conflicts(rows: &StoreRows, names: &HashMap<&str, &str>) -> Family {
    let mut family = Family::new(CONTACT_CONFLICT);
    for disagreement in disagreements(&rows.coaches, rows.school_year) {
        let subject = subject_of(
            school_of(names, disagreement.school.as_str()).unwrap_or(disagreement.school.as_str()),
            None,
        );
        family.push(queue_row(
            disagreement.school.as_str(),
            subject,
            disagreement_detail(&disagreement),
        ));
    }
    family
}

fn disagreement_detail(disagreement: &Disagreement) -> String {
    format!(
        "{}: {}; rows: {}",
        disagreement.role,
        disagreement.state.as_str(),
        disagreement.rows.join("; ")
    )
}

pub(super) fn athlete_identity(rows: &StoreRows, names: &HashMap<&str, &str>) -> Family {
    let mut family = Family::new(ATHLETE_IDENTITY);
    let mut groups: BTreeMap<IdentityKey, Vec<&CanonicalAthlete>> = BTreeMap::new();
    for athlete in class_of_2027(&rows.athletes) {
        groups
            .entry(athlete_key(athlete))
            .or_default()
            .push(athlete);
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

#[cfg(test)]
#[path = "conflicts/tests.rs"]
mod tests;
