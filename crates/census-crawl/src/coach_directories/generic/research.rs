use super::{
    CanonicalSchool, ContactResearchAttempt, FetchOutcome, Outcome, Purpose, SchoolYear, Subject,
};
use census_domain::model::ContactResearch;

pub(super) fn record_capture(
    school: &mut CanonicalSchool,
    year: SchoolYear,
    capture: &FetchOutcome,
    outcome: Outcome,
    reason: String,
) {
    record(
        school,
        year,
        ContactResearchAttempt {
            locator: locator(capture).to_owned(),
            acquired_at: capture.fetched_at.clone(),
            source_sha256: Some(capture.content_digest.clone()),
            outcome,
            reason,
        },
    );
}

pub(super) fn locator(capture: &FetchOutcome) -> &str {
    capture
        .response_url
        .as_deref()
        .map_or(capture.url.as_str(), |url| url)
}

pub(super) fn record(
    school: &mut CanonicalSchool,
    year: SchoolYear,
    attempt: ContactResearchAttempt,
) {
    let office = purpose_outcome(school, year, Purpose::SchoolOffice, &attempt);
    let athletics = purpose_outcome(school, year, Purpose::AthleticsOffice, &attempt);
    let mut office_attempt = attempt.clone();
    office_attempt.outcome = office;
    append(school, year, Purpose::SchoolOffice, office_attempt);
    let mut athletics_attempt = attempt;
    athletics_attempt.outcome = athletics;
    append(school, year, Purpose::AthleticsOffice, athletics_attempt);
}

pub(super) fn append(
    school: &mut CanonicalSchool,
    year: SchoolYear,
    purpose: Purpose,
    attempt: ContactResearchAttempt,
) {
    let research = ContactResearch {
        school: school.id.clone(),
        subject: Subject::SchoolMailbox(purpose),
        school_year: year,
        outcome: attempt.outcome.clone(),
        attempts: vec![attempt],
    };
    if !school.contact_research.contains(&research) {
        school.contact_research.push(research);
    }
}

pub(super) fn purpose_outcome(
    school: &CanonicalSchool,
    year: SchoolYear,
    purpose: Purpose,
    attempt: &ContactResearchAttempt,
) -> Outcome {
    if !attempt.outcome.is_terminal() {
        return attempt.outcome.clone();
    }
    if conflicting(school, year, purpose) {
        return Outcome::Conflict;
    }
    if school.mailbox_claims.iter().any(|claim| {
        claim.purpose == purpose
            && claim.school_year == year
            && claim.source.url.as_deref() == Some(attempt.locator.as_str())
            && Some(claim.source_sha256.as_str()) == attempt.source_sha256.as_deref()
            && claim.acquired_at == attempt.acquired_at
    }) {
        Outcome::CompletedClaims
    } else {
        Outcome::CompletedEmpty
    }
}

fn conflicting(school: &CanonicalSchool, year: SchoolYear, purpose: Purpose) -> bool {
    let mut claims = school.mailbox_claims.iter().filter(|claim| {
        claim.school_year == year
            && claim.purpose == purpose
            && claim.acquired_at.get(..10).and_then(SchoolYear::from_date) == Some(year)
    });
    let Some(first) = claims.next() else {
        return false;
    };
    claims.any(|claim| {
        !first
            .mailbox
            .trim()
            .eq_ignore_ascii_case(claim.mailbox.trim())
    })
}

pub(super) fn unattempted(school: &mut CanonicalSchool, year: SchoolYear) {
    for purpose in [Purpose::SchoolOffice, Purpose::AthleticsOffice] {
        let research = ContactResearch {
            school: school.id.clone(),
            subject: Subject::SchoolMailbox(purpose),
            school_year: year,
            outcome: Outcome::Unattempted,
            attempts: Vec::new(),
        };
        if !school.contact_research.contains(&research) {
            school.contact_research.push(research);
        }
    }
}
