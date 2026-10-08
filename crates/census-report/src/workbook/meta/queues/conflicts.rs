use census_domain::model::{CanonicalAthlete, CanonicalSchool, MEET_STATE_UNRESOLVED};
use std::collections::{BTreeMap, HashMap};

use crate::workbook::recruiting::{disagreements, Disagreement};
use census_review::athlete_flags::{key as athlete_key, IdentityKey};

use super::super::{school_of, subject_of, Family, QueueRow, StoreRows};
use super::{queue_row, ATHLETE_IDENTITY, COHORT_EVIDENCE, CONTACT_CONFLICT, SCHOOL_IDENTITY};

pub(super) fn cohort_evidence(cohort: &[CanonicalAthlete], names: &HashMap<&str, &str>) -> Family {
    let mut family = Family::new(COHORT_EVIDENCE);
    for athlete in cohort {
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
    let canonical = athlete.grad_year;
    let mut implied: Vec<String> = Vec::new();
    let mut observations: Vec<String> = Vec::new();
    let mut any_disagree = athlete.has_cohort_conflict();
    graduate_claims(athlete, &mut implied, &mut observations);
    let grades_disagree = grade_claims(athlete, &mut implied, &mut observations);
    any_disagree |= grades_disagree;
    any_disagree.then(|| {
        format!(
            "canonical grad year {}, observations imply {}: {}",
            canonical,
            implied.join("/"),
            observations.join("; ")
        )
    })
}

fn graduate_claims(
    athlete: &CanonicalAthlete,
    implied: &mut Vec<String>,
    observations: &mut Vec<String>,
) {
    for observation in &athlete.published_graduations {
        let year = observation.grad_year.to_string();
        if !implied.contains(&year) {
            implied.push(year);
        }
        observations.push(format!(
            "published graduation {} from {}",
            observation.grad_year, observation.source.id
        ));
    }
}

fn grade_claims(
    athlete: &CanonicalAthlete,
    implied: &mut Vec<String>,
    observations: &mut Vec<String>,
) -> bool {
    let mut any_disagree = false;
    for observation in &athlete.observed_grades {
        any_disagree |= record_grade(observation.grad_year(), athlete.grad_year, implied);
        observations.push(format!(
            "grade {} in {} from {}",
            observation.grade,
            observation.school_year.short(),
            observation.source.id
        ));
    }
    any_disagree
}

fn record_grade(
    year: Option<census_domain::model::GradYear>,
    canonical: census_domain::model::GradYear,
    implied: &mut Vec<String>,
) -> bool {
    let label = year.map_or_else(
        || "(outside supported range)".to_string(),
        |year| year.to_string(),
    );
    if !implied.contains(&label) {
        implied.push(label);
    }
    year != Some(canonical)
}

pub(super) fn contact_conflicts(rows: &StoreRows, names: &HashMap<&str, &str>) -> Family {
    let mut family = Family::new(CONTACT_CONFLICT);
    for disagreement in disagreements(rows.coaches, rows.school_year) {
        let subject = subject_of(
            school_of(names, disagreement.school.as_str())
                .map_or(disagreement.school.as_str(), |value| value),
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

pub(super) fn athlete_identity(cohort: &[CanonicalAthlete], names: &HashMap<&str, &str>) -> Family {
    let mut family = Family::new(ATHLETE_IDENTITY);
    let groups = athlete_groups(cohort);
    for group in groups.values().filter(|group| group.len() > 1) {
        family.group(athlete_identity_rows(group, names));
    }
    family
}

fn athlete_groups(cohort: &[CanonicalAthlete]) -> BTreeMap<IdentityKey, Vec<&CanonicalAthlete>> {
    let mut groups: BTreeMap<IdentityKey, Vec<&CanonicalAthlete>> = BTreeMap::new();
    for athlete in cohort {
        groups
            .entry(athlete_key(athlete))
            .or_default()
            .push(athlete);
    }
    groups
}
fn athlete_identity_rows(
    group: &[&CanonicalAthlete],
    names: &HashMap<&str, &str>,
) -> Vec<QueueRow> {
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
    rows
}

pub(super) fn school_identity(schools: &[CanonicalSchool]) -> Family {
    let mut family = Family::new(SCHOOL_IDENTITY);
    let groups = school_groups(schools);
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

fn school_groups(schools: &[CanonicalSchool]) -> BTreeMap<(String, String), Vec<&CanonicalSchool>> {
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
    groups
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
