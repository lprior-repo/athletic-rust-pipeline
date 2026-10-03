use super::*;
use crate::net::FetchOutcome;
use census_domain::model::Sport;

type TestResult = Result<(), Box<dyn std::error::Error>>;

mod collect;
mod mapping;

const SPORTS_FETCHED: &str = "2026-09-01T10:00:00Z";
const AD_FETCHED: &str = "2026-09-02T11:00:00Z";

fn capture(url: String, body: &str, fetched_at: &str) -> FetchOutcome {
    FetchOutcome {
        url,
        response_url: None,
        method: "GET".to_string(),
        status: 200,
        content_digest: crate::net::cache::content_digest(body.as_bytes()),
        bytes: body.len(),
        fetched_at: fetched_at.to_string(),
        from_cache: true,
        content_type: Some("text/html".to_string()),
        body: body.as_bytes().to_vec(),
    }
}

fn fixture_search_dublin() -> &'static str {
    include_str!("../../tests/fixtures/ohsaa/search_dublin_coffman.html")
}

fn fixture_sports_dublin() -> &'static str {
    include_str!("../../tests/fixtures/ohsaa/sports_dublin_coffman.html")
}

fn fixture_sports_centerville() -> &'static str {
    include_str!("../../tests/fixtures/ohsaa/sports_centerville.html")
}

fn fixture_ad_dublin() -> &'static str {
    include_str!("../../tests/fixtures/ohsaa/ad_dublin_coffman.html")
}

fn fixture_ad_centerville() -> &'static str {
    include_str!("../../tests/fixtures/ohsaa/ad_centerville.html")
}

fn fixture_search_duplicates() -> &'static str {
    include_str!("../../tests/fixtures/ohsaa/search_duplicate_rows.html")
}

fn fixture_search_no_results() -> &'static str {
    include_str!("../../tests/fixtures/ohsaa/search_no_results.html")
}

fn fixture_sports_malformed() -> &'static str {
    include_str!("../../tests/fixtures/ohsaa/sports_malformed.html")
}

fn fixture_ad_malformed() -> &'static str {
    include_str!("../../tests/fixtures/ohsaa/ad_malformed.html")
}

#[test]
fn parse_search_returns_unique_schools() {
    let results = parse_search(fixture_search_dublin());
    assert_eq!(results.len(), 1, "should have exactly 1 unique school");
    let r = &results[0];
    assert_eq!(r.name, "DUBLIN COFFMAN", "school name should be ALL-CAPS");
    assert_eq!(r.city, "Dublin", "city should be title-case");
    assert_eq!(r.ohsaa_id, "474", "should match the ohsaaId");
}

#[test]
fn parse_search_deduplicates_rows() {
    let results = parse_search(fixture_search_duplicates());
    assert_eq!(
        results.len(),
        1,
        "all duplicate rows should be deduplicated to 1"
    );
    let r = &results[0];
    assert_eq!(r.name, "MENTOR");
    assert_eq!(r.ohsaa_id, "1016");
}

#[test]
fn parse_search_empty_yields_empty_vec() {
    let results = parse_search(fixture_search_no_results());
    assert!(
        results.is_empty(),
        "search with no results table should return empty vec, not error"
    );
}

#[test]
fn parse_sports_table_extracts_xc_coaches() -> TestResult {
    let sections = parse_sports_table(fixture_sports_dublin());
    let xc = sections
        .iter()
        .find(|(label, _, _)| label == "Cross Country");
    check!(xc.is_some(), "should find Cross Country row");
    let (_, boys, girls) = xc.ok_or("Cross Country row")?;
    let boys = boys.as_ref().ok_or("boys Cross Country coach")?;
    check!(eq; boys.name, "Joe DePalma");
    check!(eq;
        boys.email,
        Some("depalma_joseph@dublinschools.net".to_string())
    );
    let girls = girls.as_ref().ok_or("girls Cross Country coach")?;
    check!(eq; girls.name, "Greg King");
    check!(eq; girls.email, Some("king_greg@dublinschools.net".to_string()));
    Ok(())
}

#[test]
fn parse_sports_table_extracts_track_field() -> TestResult {
    let sections = parse_sports_table(fixture_sports_dublin());
    let tf = sections
        .iter()
        .find(|(label, _, _)| label == "Track & Field");
    check!(tf.is_some(), "should find Track & Field row");
    let (_, boys, girls) = tf.ok_or("Track & Field row")?;
    let boys = boys.as_ref().ok_or("boys Track & Field coach")?;
    check!(eq; boys.name, "James Legins");
    check!(eq; boys.email, Some("j.legins106@gmail.com".to_string()));
    let girls = girls.as_ref().ok_or("girls Track & Field coach")?;
    check!(eq; girls.name, "Greg King");
    check!(eq; girls.email, Some("king_greg@dublinschools.net".to_string()));
    Ok(())
}

#[test]
fn parse_sports_table_handles_tba() -> TestResult {
    let sections = parse_sports_table(fixture_sports_centerville());
    let tf = sections
        .iter()
        .find(|(label, _, _)| label == "Track & Field");
    check!(tf.is_some(), "should find Track & Field row");
    let (_, boys, girls) = tf.ok_or("Track & Field row")?;
    check!(boys.is_some(), "boys coach should be present");
    let boys = boys.as_ref().ok_or("Centerville boys coach")?;
    check!(eq; boys.name, "Matt Somerlot");
    check!(eq;
        boys.email,
        Some("matt.somerlot@centerville.k12.oh.us".to_string())
    );
    check!(girls.is_none(), "girls coach TBA should parse as None");
    Ok(())
}

#[test]
fn parse_sports_table_malformed_yields_empty() {
    let sections = parse_sports_table(fixture_sports_malformed());
    assert!(
        sections.is_empty(),
        "malformed HTML should yield 0 sections, not panic"
    );
}

#[test]
fn parse_coach_cell_skips_na() {
    assert!(parse_coach_cell("N/A").is_none());
    assert!(parse_coach_cell("TBA (Div-I)").is_none());
    assert!(parse_coach_cell("").is_none());
}

#[test]
fn parse_coach_cell_parses_mailto() -> TestResult {
    let cell = r#"<a href="mailto:depalma_joseph@dublinschools.net" class="fieldValue">Joe DePalma (Div-I)</a>"#;
    let coach = parse_coach_cell(cell).ok_or("published mailto coach")?;
    check!(eq; coach.name, "Joe DePalma");
    check!(eq;
        coach.email,
        Some("depalma_joseph@dublinschools.net".to_string())
    );
    Ok(())
}

#[test]
fn parse_ad_page_extracts_director() -> TestResult {
    let ad = parse_ad_page(fixture_ad_dublin());
    check!(ad.director.is_some(), "should find athletic director");
    let (name, email) = ad.director.as_ref().ok_or("Dublin athletic director")?;
    check!(eq; name, "Duane Sheldon");
    check!(eq; email, &Some("sheldon_duane@dublinschools.net".to_string()));
    Ok(())
}

#[test]
fn parse_ad_page_excludes_office_roles_from_director() {
    let ad = parse_ad_page(fixture_ad_dublin());
    assert_eq!(ad.office_roles.len(), 2, "should have 2 office roles");
    let role_names: Vec<&str> = ad.office_roles.iter().map(|(l, _)| l.as_str()).collect();
    assert!(
        role_names.contains(&"assistant athletic director"),
        "assistant AD should be in office roles"
    );
    assert!(
        role_names.contains(&"assistant athletic secretary"),
        "assistant secretary should be in office roles"
    );
    assert!(
        !role_names.contains(&"athletic director"),
        "AD must not appear in office roles list"
    );
}

#[test]
fn parse_ad_page_malformed_yields_empty() {
    let ad = parse_ad_page(fixture_ad_malformed());
    assert!(
        ad.director.is_none(),
        "malformed AD page should not produce a director"
    );
}

#[test]
fn strip_honorific_removes_prefixes() {
    assert_eq!(strip_honorific("Coach Joe DePalma"), "Joe DePalma");
    assert_eq!(strip_honorific("Mr. Barry Mink"), "Barry Mink");
    assert_eq!(strip_honorific("Mrs. Jane Smith"), "Jane Smith");
    assert_eq!(strip_honorific("Dr. Robert Wolf"), "Robert Wolf");
    assert_eq!(
        strip_honorific("Joe DePalma"),
        "Joe DePalma",
        "no-op when no honorific"
    );
}

#[test]
fn parse_sport_label_maps_correctly() {
    assert_eq!(
        parse_sport_label("Cross Country"),
        Some(Sport::CrossCountry)
    );
    assert_eq!(
        parse_sport_label("Track & Field"),
        Some(Sport::OutdoorTrack)
    );
    assert_eq!(
        parse_sport_label("Track &amp; Field"),
        Some(Sport::OutdoorTrack)
    );
    assert_eq!(parse_sport_label("Basketball"), None);
    assert_eq!(parse_sport_label(""), None);
}
