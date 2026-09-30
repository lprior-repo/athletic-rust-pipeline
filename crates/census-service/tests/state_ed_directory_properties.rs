use census_crawl::state_ed::{parse_index, parse_profile, parse_tabular};
use census_domain::school_directory::{
    CityName, DirectoryKey, Enrollment, Phone, SchoolName, StreetLine, Website,
};
use census_domain::UsJurisdiction;

const INDEX_A: &str =
    include_str!("../../census-crawl/tests/fixtures/state_ed/index_letter_a.html");
const KINGSTON: &str =
    include_str!("../../census-crawl/tests/fixtures/state_ed/profile_kingston.html");

#[test]
fn state_ed_index_lists_the_letter_a_schools() {
    let outcome = parse_index(INDEX_A).expect("parse_index should succeed");
    assert_eq!(outcome.entries().len(), 220);
    assert_eq!(
        outcome.counts().skipped,
        0,
        "every row carries an id and a name"
    );
}

#[test]
fn state_ed_index_contains_kingston_middle() {
    let outcome = parse_index(INDEX_A).expect("parse_index should succeed");
    let names: Vec<&str> = outcome
        .entries()
        .iter()
        .filter_map(|entry| entry.name().map(SchoolName::as_str))
        .collect();
    assert!(
        names.contains(&"A A KINGSTON MIDDLE SCHOOL"),
        "index should list A A KINGSTON MIDDLE SCHOOL; found {} schools",
        names.len()
    );
}

#[test]
fn state_ed_profile_kingston_extracts_identity() {
    let outcome = parse_profile(KINGSTON).expect("parse_profile should succeed");
    assert_eq!(outcome.entries().len(), 1);
    let entry = outcome.entries().first().expect("one entry");
    assert_eq!(
        entry.name().map(SchoolName::as_str),
        Some("A A KINGSTON MIDDLE SCHOOL")
    );
    match entry.key() {
        DirectoryKey::StateRecord { state, id } => {
            assert_eq!(state, &UsJurisdiction::NewYork);
            assert_eq!(id.as_str(), "800000038718");
        }
        other => panic!("expected StateRecord key, got {other:?}"),
    }
}

#[test]
fn state_ed_profile_kingston_extracts_contact_and_size() {
    let outcome = parse_profile(KINGSTON).expect("parse_profile should succeed");
    let entry = outcome.entries().first().expect("one entry");
    assert_eq!(
        entry.phone().map(Phone::as_str),
        Some("3152652000"),
        "phone holds the digits of the tel: href"
    );
    assert_eq!(
        entry.website().map(Website::as_str),
        Some("https://www.potsdamcsd.org")
    );
    assert_eq!(entry.enrollment().map(Enrollment::get), Some(393));
}

#[test]
fn state_ed_profile_kingston_extracts_maps_address() {
    let outcome = parse_profile(KINGSTON).expect("parse_profile should succeed");
    let entry = outcome.entries().first().expect("one entry");
    let address = entry.address().expect("profile should have address");
    assert_eq!(address.line1().map(StreetLine::as_str), Some("29 Leroy St"));
    assert_eq!(address.city().map(CityName::as_str), Some("Potsdam"));
    assert_eq!(address.state(), Some(UsJurisdiction::NewYork));
    assert_eq!(address.zip().map(|zip| zip.code()), Some("13676"));
}

#[test]
fn state_ed_profile_kingston_source_is_nysed() {
    let outcome = parse_profile(KINGSTON).expect("parse_profile should succeed");
    let entry = outcome.entries().first().expect("one entry");
    let labels: Vec<String> = entry.sources().iter().map(|label| label.label()).collect();
    assert!(labels.contains(&"state-ed:NY".to_string()));
}

#[test]
fn state_ed_profile_refuses_a_page_that_is_not_a_profile() {
    let error = parse_profile("<html><body><p>Nothing here</p></body></html>")
        .expect_err("a page without a NYSED title is not a profile");
    assert!(
        matches!(error, census_crawl::CrawlError::DirectoryArtifact { .. }),
        "the refusal names the artifact: {error}"
    );
}

#[test]
fn state_ed_index_reads_no_rows_from_a_profile() {
    let outcome = parse_index(KINGSTON).expect("parse_index should succeed");
    assert_eq!(outcome.entries().len(), 0);
}

#[test]
fn state_ed_profile_is_idempotent() {
    let first = parse_profile(KINGSTON).expect("parse_profile should succeed");
    let second = parse_profile(KINGSTON).expect("parse_profile should succeed");
    assert_eq!(first, second);
}

#[test]
fn state_ed_index_is_idempotent() {
    let first = parse_index(INDEX_A).expect("parse_index should succeed");
    let second = parse_index(INDEX_A).expect("parse_index should succeed");
    assert_eq!(first, second);
}

#[test]
fn state_ed_tabular_reads_documented_columns() {
    let text = "NAME,CITY,STATE,STREET,ZIP,PHONE\n\
                A A Kingston Middle School,Potsdam,NY,29 Leroy St,13676,315-265-2000\n";
    let outcome = parse_tabular(text).expect("the documented columns should read");
    let entry = outcome.entries().first().expect("one row");
    assert_eq!(
        entry.name().map(SchoolName::as_str),
        Some("A A Kingston Middle School")
    );
    let address = entry.address().expect("row carries an address");
    assert_eq!(address.city().map(CityName::as_str), Some("Potsdam"));
    assert_eq!(address.state(), Some(UsJurisdiction::NewYork));
}

#[test]
fn state_ed_tabular_refuses_an_unknown_shape() {
    let error = parse_tabular("school,city,state\nFoo,Bar,NY\n")
        .expect_err("a tabular artifact without the documented header must refuse");
    match error {
        census_crawl::CrawlError::Invariant { detail } => {
            assert!(
                detail.contains("NAME"),
                "the refusal names the column: {detail}"
            )
        }
        other => panic!("expected a shape refusal, got {other}"),
    }
}
