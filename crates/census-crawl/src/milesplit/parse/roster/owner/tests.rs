use super::*;

mod contextual;

const TEAMS: &str = include_str!("../../../../../tests/fixtures/milesplit/wi_teams_index.html");
const BODY: &str = include_str!("../../../../../tests/fixtures/milesplit/wi_roster_52649.html");

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn team() -> Result<TeamRef, Box<dyn std::error::Error>> {
    crate::milesplit::parse_team_index(TEAMS)?
        .into_iter()
        .find(|team| team.id == "52649")
        .ok_or_else(|| "captured Abbotsford team missing".into())
}

fn historical(team: &TeamRef) -> FetchOutcome {
    FetchOutcome {
        url: format!("{}/roster", team.url),
        response_url: None,
        method: "GET".to_string(),
        status: 200,
        content_digest: crate::net::cache::content_digest(BODY.as_bytes()),
        bytes: BODY.len(),
        fetched_at: "2026-09-27T00:00:00Z".to_string(),
        from_cache: true,
        content_type: Some("text/html".to_string()),
        body: BODY.as_bytes().to_vec(),
    }
}

#[test]
fn ownerless_historical_fragment_cannot_establish_roster_affiliations() -> TestResult {
    let requested = team()?;
    let capture = historical(&requested);
    check!(matches!(
        validate_capture(BODY, &requested, &capture),
        Err(CrawlError::Schema { .. })
    ));
    let parsed = crate::milesplit::parse_roster(BODY, requested)?;
    check!(eq;
        parsed
            .roster()
            .ok_or("fragment rows lost")?
            .athletes
            .first()
            .ok_or("captured athlete missing")?
            .athlete_id,
        "14399169"
    );
    Ok(())
}

#[test]
fn matching_published_owner_admits_historical_rows_without_fabricating_final_provenance(
) -> TestResult {
    let requested = team()?;
    let capture = historical(&requested);
    let body = format!(
        "<link href='{}/roster' rel='canonical'>{BODY}",
        requested.url
    );
    validate_capture(&body, &requested, &capture)?;
    let parsed = crate::milesplit::parse_roster(&body, requested.clone())?;
    let roster = parsed
        .roster()
        .ok_or("matching owner lost readable athletes")?;
    check!(eq; roster.team, requested);
    check!(eq;
        roster
            .athletes
            .first()
            .ok_or("captured athlete missing")?
            .athlete_id,
        "14399169"
    );
    Ok(())
}

#[test]
fn final_owner_binding_rejects_foreign_identity_and_jurisdiction() -> TestResult {
    let requested = team()?;
    let expected = requested_owner(&requested)?;
    match_owner(
        "https://www.milesplit.com/teams/52649/roster",
        &requested,
        expected,
        true,
    )?;
    for url in [
        "https://wi.milesplit.com/teams/26848/roster",
        "https://oh.milesplit.com/teams/52649/roster",
        "https://wi.milesplit.com/teams/052649/roster",
        "https://wi.milesplit.com/teams/52649",
        "https://milesplit.com.attacker.test/teams/52649/roster",
    ] {
        check!(
            matches!(
                match_owner(url, &requested, expected, true),
                Err(CrawlError::Schema { .. })
            ),
            "{url}"
        );
    }
    Ok(())
}

#[test]
fn every_authoritative_owner_must_match_and_comments_cannot_prove_owner() -> TestResult {
    let requested = team()?;
    let capture = historical(&requested);
    let matching = format!("<link rel='canonical' href='{}/roster'>", requested.url);
    let conflict = format!("{matching}<meta content='https://oh.milesplit.com/teams/52649/roster' property='og:url'>{BODY}");
    check!(matches!(
        validate_capture(&conflict, &requested, &capture),
        Err(CrawlError::Schema { .. })
    ));
    let foreign_json = r#"<script type="application/ld+json">{"@type":"WebPage","url":"https://oh.milesplit.com/teams/52649/roster"}</script>"#;
    check!(matches!(
        validate_capture(foreign_json, &requested, &capture),
        Err(CrawlError::Schema { .. })
    ));
    let matching_json = format!(
        r#"<script type="application/ld+json">{{"@type":"WebPage","url":"{}/roster"}}</script>"#,
        requested.url
    );
    validate_capture(&matching_json, &requested, &capture)?;
    for body in [
        format!("<!-- {matching} -->{BODY}"),
        format!("<script>const ignored = \"{matching}\";</script>{BODY}"),
        format!(
            "<script type='application/ld+json'>broken{{\"url\":\"{}/roster\"}}</script>",
            requested.url
        ),
    ] {
        check!(matches!(
            validate_capture(&body, &requested, &capture),
            Err(CrawlError::Schema { .. })
        ));
    }
    Ok(())
}

#[test]
fn unrelated_foreign_roster_navigation_and_embedded_entities_do_not_change_document_owner(
) -> TestResult {
    let requested = team()?;
    let expected = requested_owner(&requested)?;
    let document_url = format!("{}/roster", requested.url);
    match_owner(&document_url, &requested, expected, true)?;
    let body = format!(
        r#"<link rel="canonical" href="{document_url}"><a href="https://wi.milesplit.com/teams/26848/roster">Competitor</a><script type="application/ld+json">{{"@type":"WebPage","url":"{document_url}","relatedLink":{{"@type":"SportsTeam","url":"https://oh.milesplit.com/teams/26848/roster"}}}}</script>{BODY}"#
    );
    check!(validate_published(&body, &requested)?);
    let unrelated = r#"<a href="https://wi.milesplit.com/teams/26848/roster">Competitor</a><script type="application/ld+json">{"@type":"ItemList","itemListElement":{"@type":"SportsTeam","url":"https://wi.milesplit.com/teams/26848/roster"}}</script>"#;
    check!(!validate_published(unrelated, &requested)?);
    check!(matches!(
        validate_capture(unrelated, &requested, &historical(&requested)),
        Err(CrawlError::Schema { .. })
    ));
    Ok(())
}
