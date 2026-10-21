use super::*;
use census_domain::model::GradYear;
use sha2::{Digest, Sha256};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn team() -> TeamRef {
    TeamRef {
        id: "52649".into(),
        slug: "example".into(),
        url: "https://wi.milesplit.com/teams/52649-example".into(),
        name: "Example High School".into(),
        city_state: "Madison, WI".into(),
    }
}

fn body(second_year: &str) -> String {
    format!(
        "<link rel=\"canonical\" href=\"{}/roster\"><ul id=\"rosterDataset\">{}{}</ul>",
        team().url,
        row("1", "Runner, Alice", "2027"),
        row("2", "Runner, Bob", second_year),
    )
}

fn row(id: &str, name: &str, year: &str) -> String {
    format!(
        "<li class=\"athlete-row data-row\"><a href=\"https://wi.milesplit.com/athletes/{id}-runner\">{name}</a><div class=\"column-gender\">m</div><div class=\"column-grad-year\">{year}</div></li>"
    )
}

fn capture(status: u16, body: &[u8]) -> FetchOutcome {
    FetchOutcome {
        url: format!("{}/roster", team().url),
        response_url: Some(format!("{}/roster", team().url)),
        method: "GET".into(),
        status,
        content_digest: format!("{:x}", Sha256::digest(body)),
        bytes: body.len(),
        fetched_at: "2026-10-07T12:00:00Z".into(),
        from_cache: true,
        content_type: Some("text/html".into()),
        body: body.to_vec(),
    }
}

#[test]
fn complete_and_partial_captures_keep_members_rejections_and_all_original_metadata() -> TestResult {
    for (year, members, rejected) in [("2027", 2, 0), ("", 1, 1)] {
        let html = body(year);
        let original = capture(200, html.as_bytes());
        let metadata = serde_json::to_vec(&original)?;
        let expected = super::super::parse_captured_roster(&html, team(), &original)?;
        let read = RosterOutcome::from_capture(&team(), original)?;
        check!(eq; serde_json::to_vec(&read.capture)?, metadata);
        check!(eq; read.capture.body.as_slice(), html.as_bytes());
        check!(eq; read.verdict, expected);
        check!(eq; matches!(&read.verdict, RosterVerdict::Complete { .. }), rejected == 0);
        let roster = read.verdict.roster().ok_or("accepted roster prefix")?;
        check!(eq; roster.team, team());
        check!(eq; roster.athletes.len(), members);
        let alice = roster.athletes.first().ok_or("Alice source row")?;
        check!(eq; alice.athlete().athlete_id, "1");
        check!(eq; alice.athlete().name, "Alice Runner");
        check!(eq; alice.athlete().grad_year, GradYear::CO2027);
        check!(eq; read.verdict.rejections().len(), rejected);
        if rejected != 0 {
            let rejection = read
                .verdict
                .rejections()
                .first()
                .ok_or("missing-year rejection")?;
            check!(eq; rejection.kind, RosterRejectionKind::MissingGraduationYear);
            check!(eq; rejection.athlete_id.as_deref(), Some("2"));
        }
        check!(read.verdict.unfinished().is_none());
    }
    Ok(())
}

#[test]
fn malformed_owner_is_the_original_captured_parser_refusal() -> TestResult {
    let html = body("2027");
    let mut original = capture(200, html.as_bytes());
    original.response_url = Some("https://wi.milesplit.com/teams/1-other/roster".into());
    let expected = super::super::parse_captured_roster(&html, team(), &original)
        .err()
        .ok_or("captured parser must refuse foreign owner")?;
    let actual = RosterOutcome::from_capture(&team(), original)
        .err()
        .ok_or("capture adapter must refuse foreign owner")?;
    check!(matches!(&actual, CrawlError::Schema { url, .. } if url == &team().url));
    check!(eq; actual.to_string(), expected.to_string());
    Ok(())
}

#[test]
fn not_found_and_invalid_utf8_keep_exact_quarantine_without_interpreting_body() -> TestResult {
    for (status, body, reason) in [
        (404, b"".as_slice(), RosterQuarantine::NotFound),
        (404, [255, 254].as_slice(), RosterQuarantine::NotFound),
        (
            200,
            [255, 254].as_slice(),
            RosterQuarantine::InvalidEncoding,
        ),
    ] {
        let original = capture(status, body);
        let metadata = serde_json::to_vec(&original)?;
        let read = RosterOutcome::from_capture(&team(), original)?;
        check!(eq; read.verdict, RosterVerdict::Quarantined {
            reason, rejected: Vec::new(), unfinished: None,
        });
        check!(eq; serde_json::to_vec(&read.capture)?, metadata);
        check!(eq; read.capture.body.as_slice(), body);
    }
    Ok(())
}

#[test]
fn other_statuses_keep_planned_target_in_http_error() -> TestResult {
    for status in [201, 302, 403, 500] {
        let mut original = capture(status, b"uninterpreted");
        original.url = "https://different.example/actual".into();
        check!(matches!(RosterOutcome::from_capture(&team(), original),
            Err(CrawlError::Fetch(FetchError::Http { status: actual, url }))
                if actual == status && url == format!("{}/roster", team().url)));
    }
    Ok(())
}
