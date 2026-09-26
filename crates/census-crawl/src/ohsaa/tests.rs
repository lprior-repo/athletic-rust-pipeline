//! Unit tests for the `ohsaa` adapter: fixtures in, parsed rows and canonical entities out.

use super::*;
use census_domain::model::{CoachRole, Gender, Sport};

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
fn parse_sports_table_extracts_xc_coaches() {
    let sections = parse_sports_table(fixture_sports_dublin());
    let xc = sections
        .iter()
        .find(|(label, _, _)| label == "Cross Country");
    assert!(xc.is_some(), "should find Cross Country row");
    let (_, boys, girls) = xc.unwrap();
    let boys = boys.as_ref().unwrap();
    assert_eq!(boys.name, "Joe DePalma");
    assert_eq!(
        boys.email,
        Some("depalma_joseph@dublinschools.net".to_string())
    );
    let girls = girls.as_ref().unwrap();
    assert_eq!(girls.name, "Greg King");
    assert_eq!(girls.email, Some("king_greg@dublinschools.net".to_string()));
}

#[test]
fn parse_sports_table_extracts_track_field() {
    let sections = parse_sports_table(fixture_sports_dublin());
    let tf = sections
        .iter()
        .find(|(label, _, _)| label == "Track & Field");
    assert!(tf.is_some(), "should find Track & Field row");
    let (_, boys, girls) = tf.unwrap();
    let boys = boys.as_ref().unwrap();
    assert_eq!(boys.name, "James Legins");
    assert_eq!(boys.email, Some("j.legins106@gmail.com".to_string()));
    let girls = girls.as_ref().unwrap();
    assert_eq!(girls.name, "Greg King");
    assert_eq!(girls.email, Some("king_greg@dublinschools.net".to_string()));
}

#[test]
fn parse_sports_table_handles_tba() {
    let sections = parse_sports_table(fixture_sports_centerville());
    let tf = sections
        .iter()
        .find(|(label, _, _)| label == "Track & Field");
    assert!(tf.is_some(), "should find Track & Field row");
    let (_, boys, girls) = tf.unwrap();
    assert!(boys.is_some(), "boys coach should be present");
    let boys = boys.as_ref().unwrap();
    assert_eq!(boys.name, "Matt Somerlot");
    assert_eq!(
        boys.email,
        Some("matt.somerlot@centerville.k12.oh.us".to_string())
    );
    assert!(girls.is_none(), "girls coach TBA should parse as None");
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
fn parse_coach_cell_parses_mailto() {
    let cell = r#"<a href="mailto:depalma_joseph@dublinschools.net" class="fieldValue">Joe DePalma (Div-I)</a>"#;
    let coach = parse_coach_cell(cell).expect("should parse");
    assert_eq!(coach.name, "Joe DePalma");
    assert_eq!(
        coach.email,
        Some("depalma_joseph@dublinschools.net".to_string())
    );
}

#[test]
fn parse_ad_page_extracts_director() {
    let ad = parse_ad_page(fixture_ad_dublin());
    assert!(ad.director.is_some(), "should find athletic director");
    let (name, email) = ad.director.as_ref().unwrap();
    assert_eq!(name, "Duane Sheldon");
    assert_eq!(email, &Some("sheldon_duane@dublinschools.net".to_string()));
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
fn school_entities_includes_ad_and_xc_coaches() {
    let sr = SearchResult {
        name: "DUBLIN COFFMAN".to_string(),
        city: "Dublin".to_string(),
        ohsaa_id: "474".to_string(),
    };
    let extract = school_entities(
        &sr,
        fixture_sports_dublin(),
        fixture_ad_dublin(),
        "2026-09-19",
    );

    assert_eq!(extract.school.name, "DUBLIN COFFMAN");
    assert_eq!(extract.school.city, Some("Dublin".to_string()));
    assert_eq!(extract.school.association, Some("ohsaa".to_string()));

    assert_eq!(extract.coaches.len(), 5, "should have AD + 4 coach rows");

    let roles: Vec<String> = extract
        .coaches
        .iter()
        .map(|c| format!("{:?}", c.role))
        .collect();
    assert!(roles.contains(&"AthleticDirector".to_string()));

    let all_have_email = extract
        .coaches
        .iter()
        .all(|c| c.professional_email.is_some() || c.personal_email.is_some());
    assert!(
        all_have_email,
        "all 5 coaches should carry a published address"
    );
}

#[test]
fn school_entities_handles_tba_girls_coach() {
    let sr = SearchResult {
        name: "CENTERVILLE".to_string(),
        city: "Centerville".to_string(),
        ohsaa_id: "336".to_string(),
    };
    let extract = school_entities(
        &sr,
        fixture_sports_centerville(),
        fixture_ad_centerville(),
        "2026-09-19",
    );

    assert_eq!(extract.coaches.len(), 4);

    let tf_girls = extract
        .coaches
        .iter()
        .find(|c| matches!(c.sport, Some(Sport::OutdoorTrack)) && c.gender == Gender::Girls);
    assert!(tf_girls.is_none(), "T&F girls should not be present (TBA)");
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
fn office_roles_never_imported_as_coaches() {
    let sr = SearchResult {
        name: "DUBLIN COFFMAN".to_string(),
        city: "Dublin".to_string(),
        ohsaa_id: "474".to_string(),
    };
    let extract = school_entities(
        &sr,
        fixture_sports_dublin(),
        fixture_ad_dublin(),
        "2026-09-19",
    );

    let ad_names: Vec<&str> = extract
        .coaches
        .iter()
        .filter(|c| c.role == CoachRole::AthleticDirector)
        .map(|c| c.name.as_str())
        .collect();

    assert_eq!(ad_names, vec!["Duane Sheldon"]);
    assert!(
        !ad_names.contains(&"Scott Caster"),
        "Assistant AD must not be imported as AD"
    );
    assert!(
        !ad_names.contains(&"Andrea Guilliams"),
        "Assistant Secretary must not be imported as AD"
    );
}

#[test]
fn malformed_search_does_not_panic() {
    let results = parse_search("<html><body><p>No table here</p></body></html>");
    assert!(results.is_empty());
}

#[test]
fn malformed_sports_does_not_panic() {
    let sections = parse_sports_table("<html><body><p>No table</p></body></html>");
    assert!(sections.is_empty());
}

#[test]
fn malformed_ad_does_not_panic() {
    let ad = parse_ad_page("<html><body><p>No table</p></body></html>");
    assert!(ad.director.is_none());
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
