use census_crawl::tssaa::{parse_school_list, parse_school_page};
use census_domain::model::{CoachRole, Gender, Sport};
use census_domain::school_directory::{CityName, SchoolName, StateRecordId};
use census_domain::UsJurisdiction;

const LISTING: &str = include_str!("../../census-crawl/tests/fixtures/tssaa/directory_id3.html");
const DETAIL_3: &str = include_str!("../../census-crawl/tests/fixtures/tssaa/directory_id3.html");
const DETAIL_407: &str =
    include_str!("../../census-crawl/tests/fixtures/tssaa/directory_id407.html");

#[test]
fn tssaa_listing_yields_456_schools() {
    let outcome = parse_school_list(LISTING).expect("parse listing");
    assert_eq!(
        outcome.entries().len(),
        456,
        "the embedded array has 456 schools"
    );
}

#[test]
fn tssaa_listing_keeps_the_published_city_of_each_school() {
    let outcome = parse_school_list(LISTING).expect("parse listing");
    let cities: Vec<(String, UsJurisdiction)> = outcome
        .entries()
        .iter()
        .filter_map(|entry| {
            let address = entry.address()?;
            Some((address.city()?.as_str().to_string(), address.state()?))
        })
        .collect();
    assert_eq!(cities.len(), 456, "every array row publishes a location");
    assert_eq!(
        cities
            .iter()
            .filter(|(_, state)| *state == UsJurisdiction::Tennessee)
            .count(),
        455,
        "455 rows sit in Tennessee"
    );
    let out_of_state: Vec<&(String, UsJurisdiction)> = cities
        .iter()
        .filter(|(_, state)| *state != UsJurisdiction::Tennessee)
        .collect();
    assert_eq!(
        out_of_state,
        vec![&("Southaven".to_string(), UsJurisdiction::Mississippi)],
        "the one row outside Tennessee keeps the state its parenthetical publishes"
    );
    let northpoint = outcome.entries().iter().find(|entry| {
        entry
            .address()
            .and_then(|address| address.city())
            .map(CityName::as_str)
            == Some("Southaven")
    });
    assert_eq!(
        northpoint
            .and_then(|entry| entry.name())
            .map(SchoolName::as_str),
        Some("Northpoint Christian School"),
        "the parenthetical is a location, not part of the name"
    );
}

#[test]
fn tssaa_detail_id3_has_the_track_and_cross_country_coaches() {
    let id = StateRecordId::parse("3").expect("parse id");
    let read = parse_school_page(DETAIL_3, &id).expect("parse detail");
    assert_eq!(read.school.entries().len(), 1, "one school entry");
    let entry = read.school.entries().first().expect("one entry");
    assert_eq!(
        entry.name().map(SchoolName::as_str),
        Some("Alcoa High School")
    );
    assert_eq!(
        entry
            .address()
            .and_then(|address| address.city())
            .map(CityName::as_str),
        Some("Alcoa"),
        "the embedded school array states the city"
    );
    assert_eq!(
        read.coaches.len(),
        20,
        "the page carries 20 Track/Cross Country/AD coach rows"
    );
    let first = read.coaches.first().expect("one coach");
    assert_eq!(first.person, "Pam Haggard");
    assert_eq!(first.sport, Some(Sport::CrossCountry));
    assert_eq!(first.gender, Gender::Boys);
    assert_eq!(first.role, CoachRole::HeadCoach);
    assert_eq!(
        first.email.as_deref(),
        Some("phaggard@alcoaschools.net"),
        "the reversed mail_hide domain decodes to the real host"
    );
    assert_eq!(first.phone.as_deref(), Some("865-982-4631"));
}

#[test]
fn tssaa_detail_id407_has_no_coaches() {
    let id = StateRecordId::parse("407").expect("parse id");
    let read = parse_school_page(DETAIL_407, &id).expect("parse detail");
    assert_eq!(read.school.entries().len(), 1, "one school entry");
    assert!(
        read.coaches.is_empty(),
        "id 407 has no Track/XC/AD coach rows"
    );
    let entry = read.school.entries().first().expect("one entry");
    assert_eq!(
        entry.name().map(SchoolName::as_str),
        Some("Redemption School of Worship")
    );
}
