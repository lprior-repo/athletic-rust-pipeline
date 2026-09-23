//! The review families: retained rows a human or the review model still has to adjudicate.
//!
//! Five families, every one keyed on a stored field that is empty or withheld rather than on a score:
//! a class-of-2027 athlete with no grade observation at all, one whose identity confidence sits below
//! the domain's high bar, a coach whose only published address was a personal mailbox (dropped by the
//! collection contract, objective §5), a meet no source placed in a jurisdiction, and a school with
//! no jurisdiction on its row. The cohort families are scoped to the published class of 2027; the
//! meet and school families cover the whole table, because neither row carries a cohort.

use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalSchool, Confidence,
};
use std::collections::HashMap;

use super::super::{school_of, subject_of, Family, StoreRows};
use super::{
    class_of_2027, queue_row, COHORT_UNVERIFIED, LOW_CONFIDENCE, UNRESOLVED_SCHOOL,
    UNRESOLVED_VENUE, WITHHELD_MAILBOX,
};

/// Class-of-2027 athletes with no grade observation at all: the cohort they are published under is
/// unverified, so they are the gap sweep's work (§46 stage F).
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

/// Class-of-2027 athletes whose identity confidence sits below the domain's high bar: the merge
/// accepted the row, and the objective's `identity_confidence` column is what makes them reviewable.
pub(super) fn low_confidence(rows: &StoreRows, names: &HashMap<&str, &str>) -> Family {
    let mut family = Family::new(LOW_CONFIDENCE);
    for athlete in class_of_2027(&rows.athletes) {
        if athlete.identity_confidence >= Confidence::HIGH {
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
                "identity confidence {} below the high bar of {}",
                athlete.identity_confidence.get(),
                Confidence::HIGH.get()
            ),
        ));
    }
    family
}

/// Coaches whose only published address was a personal mailbox: the collection contract dropped it
/// (objective §5), so the school still has no professional contact.
pub(super) fn withheld_mailboxes(rows: &StoreRows, names: &HashMap<&str, &str>) -> Family {
    let mut family = Family::new(WITHHELD_MAILBOX);
    for coach in &rows.coaches {
        if !coach.email_withheld {
            continue;
        }
        let subject = subject_of(&coach.name, school_of(names, coach.school.as_str()));
        family.push(queue_row(
            coach.id.as_str(),
            subject,
            format!(
                "the only published address was not a professional contact; role {}",
                role_label(coach)
            ),
        ));
    }
    family
}

/// Meets no source placed in a jurisdiction: the census files them under `??` rather than guessing,
/// and the venue decision is still owed.
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

/// Schools with no jurisdiction on their row: the census buckets them under `UNKNOWN`.
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

/// How many distinct source ids a row's evidence names.
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

/// A coach's role as the sheet prints it.
fn role_label(coach: &CanonicalCoach) -> String {
    coach.role.stable_key().to_lowercase()
}
