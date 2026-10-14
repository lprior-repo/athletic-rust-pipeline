use super::{
    historical, team, validate_capture, CrawlError, FetchOutcome, TeamRef, TestResult, BODY,
};
use crate::milesplit::parse::roster::parse_captured_roster;
use crate::milesplit::{parse_roster, RosterVerdict};

const MATCHING_URL: &str = "https://wi.milesplit.com/teams/52649-abbotsford/roster";
const FOREIGN_TEAM_URL: &str =
    "https://wi.milesplit.com/teams/26848-abundant-life-christian/roster";
const FOREIGN_HOST_URL: &str = "https://attacker.test/teams/52649-abbotsford/roster";

fn synthetic_prefixed_capture(
    requested: &TeamRef,
    synthetic_prefix: &str,
    response_url: Option<&str>,
) -> (String, FetchOutcome) {
    let body = format!("{synthetic_prefix}{BODY}");
    let mut capture = historical(requested);
    capture.response_url = response_url.map(str::to_string);
    capture.content_digest = crate::net::cache::content_digest(body.as_bytes());
    capture.bytes = body.len();
    capture.body = body.as_bytes().to_vec();
    (body, capture)
}

fn assert_refused(synthetic_prefix: &str, response_url: Option<&str>) -> TestResult {
    let requested = team()?;
    let (body, capture) = synthetic_prefixed_capture(&requested, synthetic_prefix, response_url);
    check!(
        matches!(
            validate_capture(&body, &requested, &capture),
            Err(CrawlError::Schema { .. })
        ),
        "synthetic prefix: {synthetic_prefix}"
    );
    check!(
        matches!(
            parse_captured_roster(&body, requested, &capture),
            Err(CrawlError::Schema { .. })
        ),
        "synthetic prefix: {synthetic_prefix}"
    );
    Ok(())
}

fn assert_historical_rows_admitted(synthetic_prefix: &str) -> TestResult {
    let requested = team()?;
    let expected = parse_roster(BODY, requested.clone())?;
    let (body, capture) = synthetic_prefixed_capture(&requested, synthetic_prefix, None);
    validate_capture(&body, &requested, &capture)?;
    check!(eq; parse_captured_roster(&body, requested, &capture)?, expected);
    Ok(())
}

#[test]
fn style_canonical_text_cannot_establish_historical_roster_owner() -> TestResult {
    let synthetic_prefix =
        format!("<style>/* <link rel='canonical' href='{MATCHING_URL}'> */</style>");
    assert_refused(&synthetic_prefix, None)
}

#[test]
fn textarea_canonical_text_cannot_establish_historical_roster_owner() -> TestResult {
    let synthetic_prefix =
        format!("<textarea><link rel='canonical' href='{MATCHING_URL}'></textarea>");
    assert_refused(&synthetic_prefix, None)
}

#[test]
fn title_canonical_text_cannot_establish_historical_roster_owner() -> TestResult {
    let synthetic_prefix = format!("<title><link rel='canonical' href='{MATCHING_URL}'></title>");
    assert_refused(&synthetic_prefix, None)
}

#[test]
fn xmp_canonical_text_cannot_establish_historical_roster_owner() -> TestResult {
    let synthetic_prefix = format!("<xmp><link rel='canonical' href='{MATCHING_URL}'></xmp>");
    assert_refused(&synthetic_prefix, None)
}

#[test]
fn quoted_attribute_markup_cannot_establish_historical_roster_owner() -> TestResult {
    for synthetic_prefix in [
        format!(r#"<div data-qa-synthetic="<link rel='canonical' href='{MATCHING_URL}'>"></div>"#),
        format!(r#"<div data-qa-synthetic='<link rel="canonical" href="{MATCHING_URL}">'></div>"#),
    ] {
        assert_refused(&synthetic_prefix, None)?;
    }
    Ok(())
}

#[test]
fn template_markup_cannot_establish_historical_roster_owner() -> TestResult {
    let synthetic_prefix = format!(
        "<template><template><link rel='canonical' href='{MATCHING_URL}'></template></template>"
    );
    assert_refused(&synthetic_prefix, None)
}

#[test]
fn noscript_markup_cannot_establish_historical_roster_owner() -> TestResult {
    let synthetic_prefix =
        format!("<noscript><link rel='canonical' href='{MATCHING_URL}'></noscript>");
    assert_refused(&synthetic_prefix, None)
}

#[test]
fn comments_and_script_strings_cannot_establish_historical_roster_owner() -> TestResult {
    for synthetic_prefix in [
        format!("<!-- <link rel='canonical' href='{MATCHING_URL}'> -->"),
        format!(
            r#"<script>const synthetic = "<link rel='canonical' href='{MATCHING_URL}'>";</script>"#
        ),
    ] {
        assert_refused(&synthetic_prefix, None)?;
    }
    Ok(())
}

#[test]
fn foreign_comment_and_script_strings_cannot_replace_actual_matching_owner() -> TestResult {
    for synthetic_decoy in [
        format!("<!-- <link rel='canonical' href='{FOREIGN_TEAM_URL}'> -->"),
        format!(
            r#"<script>const synthetic = "<link rel='canonical' href='{FOREIGN_HOST_URL}'>";</script>"#
        ),
    ] {
        let actual_owner = format!("<link rel='canonical' href='{MATCHING_URL}'>");
        assert_historical_rows_admitted(&format!("{synthetic_decoy}{actual_owner}"))?;
        assert_historical_rows_admitted(&format!("{actual_owner}{synthetic_decoy}"))?;
    }
    Ok(())
}

#[test]
fn matching_comment_and_script_strings_cannot_replace_actual_foreign_owner() -> TestResult {
    for synthetic_decoy in [
        format!("<!-- <link rel='canonical' href='{MATCHING_URL}'> -->"),
        format!(
            r#"<script>const synthetic = "<link rel='canonical' href='{MATCHING_URL}'>";</script>"#
        ),
    ] {
        let actual_owner = format!("<link rel='canonical' href='{FOREIGN_TEAM_URL}'>");
        assert_refused(
            &format!("{synthetic_decoy}{actual_owner}"),
            Some(MATCHING_URL),
        )?;
        assert_refused(
            &format!("{actual_owner}{synthetic_decoy}"),
            Some(MATCHING_URL),
        )?;
    }
    Ok(())
}

#[test]
fn encoded_canonical_foreign_team_overrides_matching_observed_final_url() -> TestResult {
    let synthetic_prefix = format!("<link rel='canon&#105;cal' href='{FOREIGN_TEAM_URL}'>");
    assert_refused(&synthetic_prefix, Some(MATCHING_URL))
}

#[test]
fn encoded_canonical_foreign_host_overrides_matching_observed_final_url() -> TestResult {
    let synthetic_prefix = format!("<link rel='canon&#105;cal' href='{FOREIGN_HOST_URL}'>");
    assert_refused(&synthetic_prefix, Some(MATCHING_URL))
}

#[test]
fn encoded_open_graph_foreign_team_overrides_matching_observed_final_url() -> TestResult {
    for selector in ["property", "name"] {
        let synthetic_prefix =
            format!("<meta {selector}='og&#58;url' content='{FOREIGN_TEAM_URL}'>");
        assert_refused(&synthetic_prefix, Some(MATCHING_URL))?;
    }
    Ok(())
}

#[test]
fn encoded_open_graph_foreign_host_overrides_matching_observed_final_url() -> TestResult {
    for selector in ["property", "name"] {
        let synthetic_prefix =
            format!("<meta {selector}='og&#58;url' content='{FOREIGN_HOST_URL}'>");
        assert_refused(&synthetic_prefix, Some(MATCHING_URL))?;
    }
    Ok(())
}

#[test]
fn encoded_foreign_declarations_cannot_hide_behind_matching_historical_owner() -> TestResult {
    let actual_owner = format!("<link rel='canonical' href='{MATCHING_URL}'>");
    for synthetic_conflict in [
        format!("<link rel='canon&#105;cal' href='{FOREIGN_TEAM_URL}'>"),
        format!("<meta property='og&#58;url' content='{FOREIGN_HOST_URL}'>"),
    ] {
        assert_refused(&format!("{actual_owner}{synthetic_conflict}"), None)?;
        assert_refused(&format!("{synthetic_conflict}{actual_owner}"), None)?;
    }
    Ok(())
}

#[test]
fn actual_encoded_canonical_establishes_historical_roster_owner() -> TestResult {
    let synthetic_prefix = format!("<link rel='canon&#105;cal' href='{MATCHING_URL}'>");
    assert_historical_rows_admitted(&synthetic_prefix)
}

#[test]
fn actual_encoded_open_graph_establishes_historical_roster_owner() -> TestResult {
    for selector in ["property", "name"] {
        let synthetic_prefix = format!("<meta {selector}='og&#58;url' content='{MATCHING_URL}'>");
        assert_historical_rows_admitted(&synthetic_prefix)?;
    }
    Ok(())
}

#[test]
fn encoded_matching_declaration_inside_raw_text_still_cannot_establish_owner() -> TestResult {
    for synthetic_prefix in [
        format!("<style><link rel='canon&#105;cal' href='{MATCHING_URL}'></style>"),
        format!("<textarea><meta property='og&#58;url' content='{MATCHING_URL}'></textarea>"),
        format!("<template><link rel='canon&#105;cal' href='{MATCHING_URL}'></template>"),
    ] {
        assert_refused(&synthetic_prefix, None)?;
    }
    Ok(())
}

#[test]
fn ownerless_authentic_rows_remain_available_to_pure_parser_only() -> TestResult {
    let requested = team()?;
    let capture = historical(&requested);
    check!(matches!(
        parse_captured_roster(BODY, requested.clone(), &capture),
        Err(CrawlError::Schema { .. })
    ));
    let parsed = parse_roster(BODY, requested.clone())?;
    check!(matches!(&parsed, RosterVerdict::Complete { .. }));
    let roster = parsed.roster().ok_or("authentic roster rows unavailable")?;
    check!(eq; roster.team, requested);
    check!(eq; roster.athletes.len(), 25);
    check!(eq;
        roster
            .athletes
            .first()
            .ok_or("authentic first athlete unavailable")?
            .athlete_id,
        "14399169"
    );
    Ok(())
}
