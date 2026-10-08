use super::super::map::Capture;
use super::super::parse::StaffMember;
use crate::{CrawlError, CrawlResult};
use census_domain::model::{
    validate_tenure_evidence, CanonicalCoach, CoachContactClaim, CoachContactProgram, CoachRole,
    CoachTenure, CoachTenureEvidence, SchoolYear, SourceRef,
};

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
    if coach.name.trim().is_empty() {
        return Ok(None);
    }
    let Some(claim_program) = appointment_program(coach) else {
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
    if capture.url.trim().is_empty() {
        return Err(invalid_claim(capture, "tenure evidence has no source URL"));
    }
    let tenure = tenure_for(
        scope_year(role, program),
        super::super::row::is_former(role),
        school_year,
    );
    let evidence = bound_evidence(
        coach,
        claim_program,
        tenure,
        statement(coach, role, program, capture)?,
        capture,
    );
    validate_tenure_evidence(&evidence)
        .map_err(|error| invalid_claim(capture, &error.to_string()))?;
    Ok(Some(evidence))
}

fn bound_evidence(
    coach: &CanonicalCoach,
    program: CoachContactProgram,
    tenure: CoachTenure,
    statement: String,
    capture: Capture<'_>,
) -> CoachTenureEvidence {
    CoachTenureEvidence {
        tenure,
        source: SourceRef::new(super::super::SOURCE_ID, Some(capture.url.to_owned())),
        source_sha256: capture.sha256.to_owned(),
        retrieved_at: capture.observed_on.to_owned(),
        statement,
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
