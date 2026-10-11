use super::super::map::CoachEmission;
use crate::net::FetchOutcome;
use census_domain::model::{
    CanonicalSchool, CoachContactProgram, ContactResearch, ContactResearchAttempt,
    ContactResearchOutcome as Outcome, ContactResearchSubject as Subject, SchoolYear,
};

pub(super) fn retain(
    school: &mut CanonicalSchool,
    year: SchoolYear,
    attempt: ContactResearchAttempt,
) {
    for program in ContactResearch::programs() {
        append(school, year, program, attempt.clone());
    }
}

pub(super) fn completed(
    school: &mut CanonicalSchool,
    year: SchoolYear,
    capture: &FetchOutcome,
    emission: &CoachEmission,
) {
    let baseline = if emission.counters.dropped_person > 0 || emission.counters.dropped_vendor > 0 {
        Outcome::Partial
    } else {
        Outcome::CompletedEmpty
    };
    super::super::staff_capture(school, year, capture, &emission.coaches, baseline);
}

pub(super) fn attempt(
    capture: &FetchOutcome,
    outcome: Outcome,
    reason: String,
) -> ContactResearchAttempt {
    ContactResearchAttempt {
        locator: capture
            .response_url
            .as_deref()
            .map_or(capture.url.as_str(), |url| url)
            .to_owned(),
        acquired_at: capture.fetched_at.clone(),
        source_sha256: Some(capture.content_digest.clone()),
        outcome,
        reason,
    }
}

fn append(
    school: &mut CanonicalSchool,
    year: SchoolYear,
    program: CoachContactProgram,
    attempt: ContactResearchAttempt,
) {
    let row = ContactResearch {
        school: school.id.clone(),
        subject: Subject::Program(program),
        school_year: year,
        outcome: attempt.outcome.clone(),
        attempts: vec![attempt],
    };
    if !school.contact_research.contains(&row) {
        school.contact_research.push(row);
    }
}
