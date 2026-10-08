use super::*;
use census_domain::model::{
    school_contact_research, school_mailbox, school_mailbox_research, CoachContactProgram,
    CoachRole, CoachTenure,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
const STAFF: &[u8] =
    include_bytes!("../../../tests/fixtures/coach_directories/nc_staff_summary_zcum49.json");

fn school() -> CanonicalSchool {
    let (mut school, _) = CanonicalSchool::new(
        census_domain::UsJurisdiction::NorthCarolina,
        "A.C. Reynolds High School",
        normalize_name("A.C. Reynolds High School"),
        Some("Asheville"),
    );
    school.school_website = Some("https://school.example/".to_owned());
    school
}

fn capture(body: &[u8]) -> FetchOutcome {
    FetchOutcome {
        url: "https://school.example/contact".to_owned(),
        response_url: None,
        method: "GET".to_owned(),
        status: 200,
        content_digest: crate::net::cache::content_digest(body),
        bytes: body.len(),
        fetched_at: "2026-10-07T12:34:56Z".to_owned(),
        from_cache: true,
        content_type: Some("text/html".to_owned()),
        body: body.to_vec(),
    }
}

fn offices() -> Vec<u8> {
    br#"<meta property="og:site_name" content="A.C. Reynolds High School"><p>School office: <a href="mailto:office@school.example">office@school.example</a></p><p>Athletics office: <a href="mailto:athletics@school.example">athletics@school.example</a></p>"#.to_vec()
}

#[test]
fn three_source_mailbox_purposes_never_substitute_for_each_other() -> TestResult {
    let mut school = school();
    let year = SchoolYear::new(2026).ok_or("year")?;
    let summary = crate::coach_directories::parse_summary(STAFF)?;
    let emission = crate::coach_directories::coach_entities(
        &summary,
        &school.id,
        "https://maxinfosite-api-live.dragonflyathletics.com/schools/ZCUM49/summary",
        "2026-10-07T12:34:56Z",
        year,
        &crate::net::cache::content_digest(STAFF),
    )?;
    let director = emission
        .coaches
        .iter()
        .find(|coach| coach.role == CoachRole::AthleticDirector && coach.name == "Steve Mccurry")
        .ok_or("captured director")?;
    let captured = capture(&offices());
    crate::school_sites::apply_contact_capture(&mut school, year, &captured)?;
    check!(eq; school_mailbox(&school, Purpose::SchoolOffice, year)?.map(|claim| claim.mailbox.as_str()), Some("office@school.example"));
    check!(eq; school_mailbox(&school, Purpose::AthleticsOffice, year)?.map(|claim| claim.mailbox.as_str()), Some("athletics@school.example"));
    check!(eq; director.tenure_state(year)?, CoachTenure::Current { school_year: year });
    let evidence = director
        .tenure_evidence
        .first()
        .ok_or("page-bound appointment")?;
    check!(eq; evidence.claim.as_ref().and_then(|claim| claim.mailbox.as_deref()), Some("charles.mccurry@bcsemail.org"));
    check!(eq; evidence.source_sha256, crate::net::cache::content_digest(STAFF));
    check!(eq; evidence.retrieved_at, "2026-10-07T12:34:56Z");
    check!(eq; school_contact_research(&school, &CoachContactProgram::SchoolAthletics, year), Outcome::CompletedEmpty);
    Ok(())
}

#[test]
fn malformed_school_office_never_poison_independent_athletics_office() -> TestResult {
    let year = SchoolYear::new(2026).ok_or("year")?;
    let body =
        String::from_utf8(offices())?.replace("office@school.example", "bad@@school.example");
    let mut school = school();
    apply_school_mailbox_capture(&mut school, year, &capture(body.as_bytes()))?;
    check!(eq; school_mailbox_research(&school, Purpose::SchoolOffice, year), Outcome::Partial);
    check!(eq; school_mailbox_research(&school, Purpose::AthleticsOffice, year), Outcome::CompletedClaims);
    check!(eq; school_mailbox(&school, Purpose::AthleticsOffice, year)?.map(|claim| claim.mailbox.as_str()), Some("athletics@school.example"));
    Ok(())
}

#[test]
fn contact_body_budget_preserves_earlier_admitted_mailbox_without_certifying_completion(
) -> TestResult {
    let year = SchoolYear::new(2026).ok_or("year")?;
    let body = format!(
        "{}<p>{}</p>",
        String::from_utf8(offices())?,
        "x".repeat(4097)
    );
    let mut school = school();
    apply_school_mailbox_capture(&mut school, year, &capture(body.as_bytes()))?;
    let claim = school
        .mailbox_claims
        .iter()
        .find(|claim| claim.purpose == Purpose::SchoolOffice)
        .ok_or("admitted prefix")?;
    check!(eq; claim.mailbox, "office@school.example");
    check!(eq; school_mailbox_research(&school, Purpose::SchoolOffice, year), Outcome::Partial);
    Ok(())
}

#[test]
fn source_research_failure_empty_partial_and_blocked_keep_exact_capture() -> TestResult {
    let year = SchoolYear::new(2026).ok_or("year")?;
    for (body, status, expected) in [
        (offices(), 200, Outcome::CompletedEmpty),
        (offices(), 403, Outcome::Blocked),
        (vec![0xff], 200, Outcome::Failed),
    ] {
        let mut school = school();
        let mut captured = capture(&body);
        captured.status = status;
        crate::school_sites::apply_contact_capture(&mut school, year, &captured)?;
        check!(eq; school_contact_research(&school, &CoachContactProgram::SchoolAthletics, year), expected);
        let row = school
            .contact_research
            .iter()
            .find(|row| row.subject == Subject::Program(CoachContactProgram::SchoolAthletics))
            .ok_or("source program outcome")?;
        let attempt = row.attempts.first().ok_or("capture attempt")?;
        check!(eq; attempt.locator, captured.url);
        check!(eq; attempt.acquired_at, captured.fetched_at);
        check!(eq; attempt.source_sha256.as_deref(), Some(captured.content_digest.as_str()));
    }
    Ok(())
}

#[test]
fn missing_or_invalid_capture_time_cannot_certify_empty_current_research() -> TestResult {
    let year = SchoolYear::new(2026).ok_or("year")?;
    for timestamp in ["", "2026-10-07broken"] {
        let mut school = school();
        let mut captured = capture(&offices());
        captured.fetched_at = timestamp.to_owned();
        crate::school_sites::apply_contact_capture(&mut school, year, &captured)?;
        check!(eq; school_mailbox_research(&school, Purpose::SchoolOffice, year), Outcome::Failed);
        check!(eq; school_mailbox(&school, Purpose::SchoolOffice, year)?, None);
        check!(eq; school_contact_research(&school, &CoachContactProgram::SchoolAthletics, year), Outcome::Failed);
    }
    Ok(())
}
