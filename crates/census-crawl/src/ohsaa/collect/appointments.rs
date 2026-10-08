use crate::net::FetchOutcome;
use crate::{CrawlError, CrawlResult};
use census_domain::model::{
    validate_tenure_evidence, CanonicalCoach, CoachContactClaim, CoachContactProgram, CoachRole,
    CoachTenure, CoachTenureEvidence, SchoolYear, SourceRef,
};

pub(super) fn qualify(coach: &mut CanonicalCoach, capture: &FetchOutcome) -> CrawlResult<()> {
    let year = capture
        .fetched_at
        .get(..10)
        .and_then(SchoolYear::from_date)
        .ok_or_else(|| invalid(capture, "appointment capture has no academic year"))?;
    let (program, role) = program_and_role(coach, capture)?;
    let evidence = CoachTenureEvidence {
        tenure: CoachTenure::Current { school_year: year },
        source: SourceRef::new(super::super::SOURCE_ID, Some(capture.url.clone())),
        source_sha256: capture.content_digest.clone(),
        retrieved_at: capture.fetched_at.clone(),
        statement: format!("{}: {role} for {program:?}.", coach.name),
        claim: Some(CoachContactClaim {
            coach: coach.id.clone(),
            school: coach.school.clone(),
            role: coach.role,
            program,
            mailbox: coach
                .professional_email
                .as_ref()
                .or(coach.personal_email.as_ref())
                .cloned(),
        }),
    };
    validate_tenure_evidence(&evidence).map_err(|error| invalid(capture, &error.to_string()))?;
    coach.tenure_evidence.push(evidence);
    Ok(())
}

fn program_and_role(
    coach: &CanonicalCoach,
    capture: &FetchOutcome,
) -> CrawlResult<(CoachContactProgram, &'static str)> {
    match (coach.role, coach.sport) {
        (CoachRole::AthleticDirector, None) => {
            Ok((CoachContactProgram::SchoolAthletics, "Athletic Director"))
        }
        (CoachRole::HeadCoach, Some(sport)) => Ok((
            CoachContactProgram::Team {
                sport,
                gender: coach.gender,
            },
            "Head Coach",
        )),
        _ => Err(invalid(
            capture,
            "appointment has no published role and program",
        )),
    }
}

pub(super) fn current(capture: &FetchOutcome, year: SchoolYear) -> bool {
    chrono::DateTime::parse_from_rfc3339(&capture.fetched_at).is_ok()
        && capture.fetched_at.get(..10).and_then(SchoolYear::from_date) == Some(year)
}

fn invalid(capture: &FetchOutcome, detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: capture.url.clone(),
        detail: detail.to_owned(),
    }
}
