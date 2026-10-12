use super::super::map::Capture;
use super::super::parse::StaffMember;
use crate::{CrawlError, CrawlResult};
use census_domain::model::{
    validate_tenure_evidence, CanonicalCoach, CoachContactClaim, CoachContactProgram, CoachRole,
    CoachTenure, CoachTenureEvidence, SchoolYear, SourceRef,
};

pub(crate) struct PublishedListing<'a> {
    pub role: &'a str,
    pub program: &'a str,
    pub source: &'a str,
    pub school_year: SchoolYear,
}

pub(super) fn appointment(
    coach: &CanonicalCoach,
    member: &StaffMember,
    program: Option<&str>,
    capture: Capture<'_>,
    school_year: Option<SchoolYear>,
) -> CrawlResult<Option<CoachTenureEvidence>> {
    let Some(school_year) = school_year else {
        return Ok(None);
    };
    let Some((role, program)) = member
        .title
        .as_deref()
        .zip(program)
        .filter(|(role, program)| !role.trim().is_empty() && !program.trim().is_empty())
    else {
        return Ok(None);
    };
    listing_appointment(
        coach,
        PublishedListing {
            role,
            program,
            source: super::super::SOURCE_ID,
            school_year,
        },
        capture,
    )
}

pub(crate) fn listing_appointment(
    coach: &CanonicalCoach,
    listing: PublishedListing<'_>,
    capture: Capture<'_>,
) -> CrawlResult<Option<CoachTenureEvidence>> {
    if coach.name.trim().is_empty() {
        return Ok(None);
    }
    let Some(claim_program) = appointment_program(coach) else {
        return Ok(None);
    };
    let role = listing.role;
    let program = listing.program;
    if role.trim().is_empty() || program.trim().is_empty() {
        return Ok(None);
    };
    if capture.url.trim().is_empty() {
        return Err(invalid_claim(capture, "tenure evidence has no source URL"));
    }
    let tenure = tenure_for(
        scope_year(role, program),
        super::super::row::is_former(role),
        listing.school_year,
    );
    let evidence = bound_evidence(
        Some(contact_claim(coach, claim_program)),
        tenure,
        statement(coach, role, program, capture)?,
        capture,
        listing.source,
    );
    validate_tenure_evidence(&evidence)
        .map_err(|error| invalid_claim(capture, &error.to_string()))?;
    Ok(Some(evidence))
}

pub(crate) fn record_appointment(
    coach: &mut CanonicalCoach,
    listing: Option<PublishedListing<'_>>,
    capture: Capture<'_>,
) -> CrawlResult<()> {
    if let Some(listing) = listing {
        let source = listing.source;
        if let Some(evidence) = listing_appointment(coach, listing, capture)? {
            return record_evidence(coach, evidence);
        }
        return record_unknown(coach, capture, source);
    }
    let source = coach
        .evidence
        .iter()
        .find(|evidence| evidence.source.url.as_deref() == Some(capture.url))
        .map_or(super::super::SOURCE_ID, |evidence| {
            evidence.source.id.as_str()
        });
    let evidence = unknown_appointment(coach, capture, source)?;
    record_evidence(coach, evidence)
}

pub(crate) fn record_capture_appointments(
    coaches: &mut [CanonicalCoach],
    capture: &crate::net::FetchOutcome,
    source: &str,
) -> CrawlResult<()> {
    let binding = Capture {
        url: &capture.url,
        observed_on: &capture.fetched_at,
        sha256: &capture.content_digest,
    };
    for coach in coaches {
        if coach.tenure_evidence.is_empty() {
            record_unknown(coach, binding, source)?;
        }
    }
    Ok(())
}

fn record_unknown(
    coach: &mut CanonicalCoach,
    capture: Capture<'_>,
    source: &str,
) -> CrawlResult<()> {
    let evidence = unknown_appointment(coach, capture, source)?;
    record_evidence(coach, evidence)
}

fn unknown_appointment(
    coach: &CanonicalCoach,
    capture: Capture<'_>,
    source: &str,
) -> CrawlResult<CoachTenureEvidence> {
    if capture.url.trim().is_empty() {
        return Err(invalid_claim(capture, "tenure evidence has no source URL"));
    }
    let evidence = bound_evidence(
        appointment_program(coach).map(|program| contact_claim(coach, program)),
        CoachTenure::Unknown,
        "Published identity does not establish a current appointment.".to_owned(),
        capture,
        source,
    );
    validate_tenure_evidence(&evidence)
        .map_err(|error| invalid_claim(capture, &error.to_string()))?;
    Ok(evidence)
}

fn record_evidence(coach: &mut CanonicalCoach, evidence: CoachTenureEvidence) -> CrawlResult<()> {
    if !coach.tenure_evidence.contains(&evidence) {
        crate::directory::acquisition::bounded(&mut coach.tenure_evidence, evidence)?;
    }
    Ok(())
}

fn bound_evidence(
    claim: Option<CoachContactClaim>,
    tenure: CoachTenure,
    statement: String,
    capture: Capture<'_>,
    source: &str,
) -> CoachTenureEvidence {
    CoachTenureEvidence {
        tenure,
        source: SourceRef::new(source, Some(capture.url.to_owned())),
        source_sha256: capture.sha256.to_owned(),
        retrieved_at: capture.observed_on.to_owned(),
        statement,
        claim,
    }
}

fn contact_claim(coach: &CanonicalCoach, program: CoachContactProgram) -> CoachContactClaim {
    CoachContactClaim {
        coach: coach.id.clone(),
        school: coach.school.clone(),
        role: coach.role,
        program,
        mailbox: coach
            .professional_email
            .as_ref()
            .or(coach.personal_email.as_ref())
            .cloned(),
    }
}

fn appointment_program(coach: &CanonicalCoach) -> Option<CoachContactProgram> {
    match coach.role {
        CoachRole::HeadCoach | CoachRole::AssistantCoach => {
            coach.sport.map(|sport| CoachContactProgram::Team {
                sport,
                gender: coach.gender,
            })
        }
        CoachRole::AthleticDirector => Some(CoachContactProgram::SchoolAthletics),
        CoachRole::Unknown => None,
    }
}
fn scope_year(title: &str, program: &str) -> Option<SchoolYear> {
    super::super::map::season_year(title).or_else(|| super::super::map::season_year(program))
}

fn tenure_for(
    source_year: Option<SchoolYear>,
    is_former: bool,
    run_year: SchoolYear,
) -> CoachTenure {
    if is_former {
        return CoachTenure::Former {
            last_school_year: source_year,
        };
    }
    match source_year {
        Some(year) if year != run_year => CoachTenure::Former {
            last_school_year: Some(year),
        },
        Some(year) => CoachTenure::Current { school_year: year },
        None => CoachTenure::Current {
            school_year: run_year,
        },
    }
}

fn statement(
    coach: &CanonicalCoach,
    role: &str,
    program: &str,
    capture: Capture<'_>,
) -> CrawlResult<String> {
    let suffix = if coach.role == CoachRole::AthleticDirector {
        " athletics"
    } else {
        ""
    };
    let length = coach
        .name
        .len()
        .checked_add(role.len())
        .and_then(|length| length.checked_add(program.len()))
        .and_then(|length| length.checked_add(suffix.len()))
        .and_then(|length| length.checked_add(8));
    if !length.is_some_and(|length| length <= 512) {
        return Err(invalid_claim(capture, "tenure statement exceeds 512 bytes"));
    }
    Ok(format!("{}: {role} for {program}{suffix}.", coach.name))
}

fn invalid_claim(capture: Capture<'_>, detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: capture.url.to_string(),
        detail: format!("directory appointment evidence: {detail}"),
    }
}
