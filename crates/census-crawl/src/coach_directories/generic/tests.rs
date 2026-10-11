use super::*;
use census_domain::model::{school_mailbox, school_mailbox_research};
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;
const EXTRACT: &[u8] = include_bytes!("fixtures/cac-contact-extract.html");

fn school() -> CanonicalSchool {
    let (mut school, _) = CanonicalSchool::new(
        UsJurisdiction::Arkansas,
        "Central Arkansas Christian Schools",
        "central arkansas christian schools",
        None,
    );
    school.school_website = Some("https://cacmustangs.org/".to_owned());
    school
}

fn capture(body: &[u8]) -> FetchOutcome {
    FetchOutcome {
        url: "https://cacmustangs.org/about/contact/".to_owned(),
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

#[test]
fn cen07_real_contact_extract_retains_generic_capture_with_no_coach_rows() -> TestResult {
    let year = SchoolYear::new(2026).ok_or("year")?;
    let mut school = school();
    let capture = capture(EXTRACT);
    apply_school_mailbox_capture(&mut school, year, &capture)?;
    let mailbox =
        school_mailbox(&school, Purpose::SchoolOffice, year)?.ok_or("published school office")?;
    check!(eq; mailbox.mailbox, "cac@cacmustangs.org");
    check!(eq; mailbox.source.url.as_deref(), Some(capture.url.as_str()));
    check!(eq; mailbox.source_sha256, capture.content_digest);
    check!(eq; mailbox.acquired_at, "2026-10-07T12:34:56Z");
    check!(eq; school_mailbox_research(&school, Purpose::SchoolOffice, year), Outcome::CompletedClaims);
    Ok(())
}

#[test]
fn cen07_named_staff_is_not_promoted_to_generic_office() -> TestResult {
    let year = SchoolYear::new(2026).ok_or("year")?;
    let body = std::str::from_utf8(EXTRACT)?.replace(
        "Contact us anytime with questions or comments. Email us at ",
        "School office: Principal Jane Smith ",
    );
    let mut school = school();
    apply_school_mailbox_capture(&mut school, year, &capture(body.as_bytes()))?;
    check!(eq; school_mailbox(&school, Purpose::SchoolOffice, year)?, None);
    check!(eq; school_mailbox_research(&school, Purpose::SchoolOffice, year), Outcome::CompletedEmpty);
    Ok(())
}

#[test]
fn cen07_foreign_campus_owner_is_retained_ambiguous() -> TestResult {
    let year = SchoolYear::new(2026).ok_or("year")?;
    let mut school = school();
    school.normalized_name = "central arkansas christian secondary campus".to_owned();
    apply_school_mailbox_capture(&mut school, year, &capture(EXTRACT))?;
    check!(eq; school_mailbox(&school, Purpose::SchoolOffice, year)?, None);
    check!(eq; school_mailbox_research(&school, Purpose::SchoolOffice, year), Outcome::Ambiguous);
    Ok(())
}

#[test]
fn cen07_inert_markup_cannot_create_generic_office_claims() -> TestResult {
    let year = SchoolYear::new(2026).ok_or("year")?;
    for inert in [
        "template", "svg", "math", "script", "style", "textarea", "noscript",
    ] {
        let body = format!("<meta property=\"og:site_name\" content=\"Central Arkansas Christian Schools\"><{inert}>{}</{inert}>", std::str::from_utf8(EXTRACT)?);
        let mut school = school();
        apply_school_mailbox_capture(&mut school, year, &capture(body.as_bytes()))?;
        check!(eq; school_mailbox(&school, Purpose::SchoolOffice, year)?, None);
    }
    Ok(())
}

#[test]
fn cen07_unclosed_final_paragraph_does_not_drop_published_mailbox() -> TestResult {
    let year = SchoolYear::new(2026).ok_or("year")?;
    let body = std::str::from_utf8(EXTRACT)?.replace("</p>", "");
    let mut school = school();
    apply_school_mailbox_capture(&mut school, year, &capture(body.as_bytes()))?;
    check!(eq; school_mailbox(&school, Purpose::SchoolOffice, year)?.map(|claim| claim.mailbox.as_str()), Some("cac@cacmustangs.org"));
    Ok(())
}

#[test]
fn cen07_distinct_current_generic_mailboxes_withhold_current_output() -> TestResult {
    let year = SchoolYear::new(2026).ok_or("year")?;
    let mut school = school();
    apply_school_mailbox_capture(&mut school, year, &capture(EXTRACT))?;
    let body =
        std::str::from_utf8(EXTRACT)?.replace("cac@cacmustangs.org", "office@cacmustangs.org");
    let mut second = capture(body.as_bytes());
    second.fetched_at = "2026-10-08T12:34:56Z".to_owned();
    apply_school_mailbox_capture(&mut school, year, &second)?;
    check!(eq; school_mailbox(&school, Purpose::SchoolOffice, year), Err(census_domain::model::SchoolContactError::Conflict));
    check!(eq; school_mailbox_research(&school, Purpose::SchoolOffice, year), Outcome::Conflict);
    Ok(())
}

#[test]
fn cen07_office_purpose_accepts_published_gmail_capture() -> TestResult {
    let year = SchoolYear::new(2026).ok_or("year")?;
    let body =
        std::str::from_utf8(EXTRACT)?.replace("cac@cacmustangs.org", "school.office@gmail.com");
    let mut school = school();
    apply_school_mailbox_capture(&mut school, year, &capture(body.as_bytes()))?;
    check!(eq; school_mailbox(&school, Purpose::SchoolOffice, year)?.map(|claim| claim.mailbox.as_str()), Some("school.office@gmail.com"));
    Ok(())
}

#[test]
fn cen07_stale_capture_cannot_become_current_by_relabeling_request_year() -> TestResult {
    let year = SchoolYear::new(2026).ok_or("year")?;
    let mut school = school();
    let mut old = capture(EXTRACT);
    old.fetched_at = "2025-10-07T12:34:56Z".to_owned();
    apply_school_mailbox_capture(&mut school, year, &old)?;
    check!(eq; school_mailbox(&school, Purpose::SchoolOffice, year)?.map(|claim| claim.mailbox.as_str()), None);
    check!(eq; school_mailbox_research(&school, Purpose::SchoolOffice, year), Outcome::Stale);
    let body = std::str::from_utf8(EXTRACT)?
        .replace("cac@cacmustangs.org", "current.office@cacmustangs.org");
    apply_school_mailbox_capture(&mut school, year, &capture(body.as_bytes()))?;
    check!(eq; school_mailbox(&school, Purpose::SchoolOffice, year)?.map(|claim| claim.mailbox.as_str()), Some("current.office@cacmustangs.org"));
    Ok(())
}
