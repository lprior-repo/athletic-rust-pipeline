use super::{apply_contact_capture, CanonicalSchool, FetchOutcome, Outcome, SchoolYear, Subject};
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const OWNER_BLOCKER: &str = "published page owner does not uniquely match the canonical school";
const PAGE: &str = "<a href='/staff'>Staff</a><p>School office: <a href='mailto:office@school.example'>office@school.example</a></p>";

fn school() -> CanonicalSchool {
    let (mut school, _) = CanonicalSchool::new(
        UsJurisdiction::Michigan,
        "Example School",
        census_domain::model::normalize_name("Example School"),
        None,
    );
    school.school_website = Some("https://school.example/".to_owned());
    school
}

fn capture(owner: Option<&str>) -> FetchOutcome {
    let html = match owner {
        Some(owner) => format!("<meta property='og:site_name' content='{owner}'>{PAGE}"),
        None => PAGE.to_owned(),
    };
    let body = html.into_bytes();
    FetchOutcome {
        url: "https://school.example/contact".to_owned(),
        response_url: None,
        method: "GET".to_owned(),
        status: 200,
        content_digest: crate::net::cache::content_digest(&body),
        bytes: body.len(),
        fetched_at: "2026-10-07T12:34:56Z".to_owned(),
        from_cache: true,
        content_type: Some("text/html".to_owned()),
        body,
    }
}

fn check_programs(
    school: &CanonicalSchool,
    capture: &FetchOutcome,
    expected: Outcome,
    reason: &str,
) -> TestResult {
    let attempts: Vec<_> = school
        .contact_research
        .iter()
        .filter(|row| matches!(row.subject, Subject::Program(_)))
        .flat_map(|row| &row.attempts)
        .filter(|attempt| attempt.source_sha256.as_deref() == Some(capture.content_digest.as_str()))
        .collect();
    check!(!attempts.is_empty());
    for attempt in attempts {
        check!(eq; attempt.outcome, expected);
        check!(eq; attempt.reason, reason);
        check!(eq; attempt.locator, capture.url);
        check!(eq; attempt.acquired_at, capture.fetched_at);
    }
    Ok(())
}

#[test]
fn ownerless_capture_retains_program_blocker_and_discovery_links() -> TestResult {
    let mut school = school();
    let capture = capture(None);
    let original = capture.body.clone();
    let links = apply_contact_capture(&mut school, SchoolYear::new(2026).ok_or("year")?, &capture)?;
    check!(eq; links, vec!["https://school.example/staff"]);
    check!(school.mailbox_claims.is_empty());
    check!(eq; capture.body, original);
    check_programs(&school, &capture, Outcome::Ambiguous, OWNER_BLOCKER)
}

#[test]
fn later_owned_capture_does_not_inherit_skipped_program_reason() -> TestResult {
    let mut school = school();
    let year = SchoolYear::new(2026).ok_or("year")?;
    let missing = capture(None);
    apply_contact_capture(&mut school, year, &missing)?;
    let owned = capture(Some("Example School"));
    apply_contact_capture(&mut school, year, &owned)?;
    check!(eq; school.mailbox_claims.len(), 1, "owned capture should yield mailbox claim");
    check_programs(&school, &missing, Outcome::Ambiguous, OWNER_BLOCKER)?;
    check_programs(
        &school,
        &owned,
        Outcome::CompletedEmpty,
        "searched retained source page; candidate names never establish current tenure or mailbox claims",
    )
}

#[test]
fn foreign_owner_capture_still_withholds_claims_and_keeps_reason() -> TestResult {
    let mut school = school();
    let capture = capture(Some("Other School"));
    let links = apply_contact_capture(&mut school, SchoolYear::new(2026).ok_or("year")?, &capture)?;
    check!(eq; links, vec!["https://school.example/staff"]);
    check!(school.mailbox_claims.is_empty());
    check_programs(&school, &capture, Outcome::Ambiguous, OWNER_BLOCKER)
}
