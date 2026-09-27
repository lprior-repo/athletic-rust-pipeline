
use super::super::{school_of, subject_of, Family, StoreRows};
use super::{
    class_of_2027, queue_row, COHORT_UNVERIFIED, IDENTITY_UNVERIFIED, UNRESOLVED_SCHOOL,
    UNRESOLVED_VENUE,
};
use census_domain::model::{CanonicalAthlete, CanonicalMeet, CanonicalSchool};
use std::collections::HashMap;

pub(super) fn cohort_unverified(rows: &StoreRows, names: &HashMap<&str, &str>) -> Family {
    let mut family = Family::new(COHORT_UNVERIFIED);
    for athlete in class_of_2027(&rows.athletes) {
        if !athlete.observed_grades.is_empty() {
            continue;
        }
        let subject = subject_of(
            &athlete.canonical_name,
            school_of(names, athlete.school.as_str()),
        );
        family.push(queue_row(
            athlete.id.as_str(),
            subject,
            format!(
                "no grade observation retained; {} evidence row(s), {} source(s)",
                athlete.evidence.len(),
                source_count(athlete)
            ),
        ));
    }
    family
}

pub(super) fn identity_unverified(rows: &StoreRows, names: &HashMap<&str, &str>) -> crate::report::ReportResult<Family> {
    let mut family = Family::new(IDENTITY_UNVERIFIED);
    for athlete in class_of_2027(&rows.athletes) {
        let status = rows.identities.status(athlete.id.as_str())
            .map_err(census_store::StoreError::from)?;
        if status == census_domain::model::IdentityStatus::Verified {
            continue;
        }
        let subject = subject_of(
            &athlete.canonical_name,
            school_of(names, athlete.school.as_str()),
        );
        family.push(queue_row(
            athlete.id.as_str(),
            subject,
            format!(
                "identity status {}; verification requires a current admissible identity decision",
                status.as_str(),
            ),
        ));
    }
    Ok(family)
}

pub(super) fn unresolved_venues(meets: &[CanonicalMeet]) -> Family {
    let mut family = Family::new(UNRESOLVED_VENUE);
    for meet in meets {
        if meet.state.is_some() {
            continue;
        }
        family.push(queue_row(
            meet.id.as_str(),
            format!("{} ({})", meet.name, meet.date),
            format!(
                "no evidence placed the venue in a jurisdiction; filed under {}",
                census_domain::model::MEET_STATE_UNRESOLVED
            ),
        ));
    }
    family
}

pub(super) fn unresolved_schools(schools: &[CanonicalSchool]) -> Family {
    let mut family = Family::new(UNRESOLVED_SCHOOL);
    for school in schools {
        if school.state.is_some() {
            continue;
        }
        family.push(queue_row(
            school.id.as_str(),
            school.name.clone(),
            "no association or adapter placed the school in a jurisdiction".to_string(),
        ));
    }
    family
}

fn source_count(athlete: &CanonicalAthlete) -> usize {
    let mut sources: Vec<&str> = athlete
        .evidence
        .iter()
        .map(|evidence| evidence.source.id.as_str())
        .collect();
    sources.sort_unstable();
    sources.dedup();
    sources.len()
}
