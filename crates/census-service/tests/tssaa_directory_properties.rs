use census_crawl::tssaa::{parse_school_list, parse_school_page};
use census_domain::model::{CoachRole, Gender, Sport};
use census_domain::school_directory::{CityName, SchoolName, StateRecordId};
use census_domain::UsJurisdiction;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const LISTING: &str = include_str!("../../census-crawl/tests/fixtures/tssaa/directory_id3.html");
const DETAIL_3: &str = include_str!("../../census-crawl/tests/fixtures/tssaa/directory_id3.html");
const DETAIL_407: &str =
    include_str!("../../census-crawl/tests/fixtures/tssaa/directory_id407.html");

#[test]
fn tssaa_listing_yields_456_schools() -> TestResult {
    let outcome = parse_school_list(LISTING)?;
    let entries = outcome.entries().len();
    if entries != 456 {
        return Err(
            format!("the embedded array has 456 schools: left={entries}, right=456").into(),
        );
    }
    Ok(())
}

#[test]
fn tssaa_listing_keeps_the_published_city_of_each_school() -> TestResult {
    let outcome = parse_school_list(LISTING)?;
    let cities: Vec<(String, UsJurisdiction)> = outcome
        .entries()
        .iter()
        .filter_map(|entry| {
            let address = entry.address()?;
            Some((address.city()?.as_str().to_string(), address.state()?))
        })
        .collect();
    let entries = cities.len();
    if entries != 456 {
        return Err(
            format!("every array row publishes a location: left={entries}, right=456").into(),
        );
    }
    let tennessee = cities
        .iter()
        .filter(|(_, state)| *state == UsJurisdiction::Tennessee)
        .count();
    if tennessee != 455 {
        return Err(format!("455 rows sit in Tennessee: left={tennessee}, right=455").into());
    }
    let out_of_state: Vec<&(String, UsJurisdiction)> = cities
        .iter()
        .filter(|(_, state)| *state != UsJurisdiction::Tennessee)
        .collect();
    let expected_out_of_state = ("Southaven".to_string(), UsJurisdiction::Mississippi);
    let expected_out_of_state = vec![&expected_out_of_state];
    if out_of_state != expected_out_of_state {
        return Err(format!("the one row outside Tennessee keeps the state its parenthetical publishes: left={out_of_state:?}, right={expected_out_of_state:?}").into());
    }
    let northpoint = outcome.entries().iter().find(|entry| {
        entry
            .address()
            .and_then(|address| address.city())
            .map(CityName::as_str)
            == Some("Southaven")
    });
    let name = northpoint
        .and_then(|entry| entry.name())
        .map(SchoolName::as_str);
    if name != Some("Northpoint Christian School") {
        return Err(format!(
            "the parenthetical is a location, not part of the name: left={name:?}, right={:?}",
            Some("Northpoint Christian School")
        )
        .into());
    }
    Ok(())
}

#[test]
fn tssaa_detail_id3_has_the_track_and_cross_country_coaches() -> TestResult {
    let id = StateRecordId::parse("3")?;
    let read = parse_school_page(DETAIL_3, &id)?;
    let entries = read.school.entries().len();
    if entries != 1 {
        return Err(format!("one school entry: left={entries}, right=1").into());
    }
    let entry = read.school.entries().first().ok_or("missing entry")?;
    let name = entry.name().map(SchoolName::as_str);
    if name != Some("Alcoa High School") {
        return Err(format!(
            "school name: left={name:?}, right={:?}",
            Some("Alcoa High School")
        )
        .into());
    }
    let city = entry
        .address()
        .and_then(|address| address.city())
        .map(CityName::as_str);
    if city != Some("Alcoa") {
        return Err(format!(
            "the embedded school array states the city: left={city:?}, right={:?}",
            Some("Alcoa")
        )
        .into());
    }
    let pam = read
        .coaches
        .iter()
        .find(|coach| {
            coach.person == "Pam Haggard"
                && coach.sport == Some(Sport::CrossCountry)
                && coach.gender == Gender::Boys
        })
        .ok_or("missing boys cross-country appointment")?;
    if pam.role != CoachRole::HeadCoach {
        return Err(format!(
            "coach role: left={:?}, right={:?}",
            pam.role,
            CoachRole::HeadCoach
        )
        .into());
    }
    let email = pam.email.as_deref();
    if email != Some("phaggard@alcoaschools.net") {
        return Err(format!(
            "the reversed mail_hide domain decodes to the real host: left={email:?}, right={:?}",
            Some("phaggard@alcoaschools.net")
        )
        .into());
    }
    let phone = pam.phone.as_deref();
    if phone != Some("865-982-4631") {
        return Err(format!(
            "coach phone: left={phone:?}, right={:?}",
            Some("865-982-4631")
        )
        .into());
    }
    let director = read
        .coaches
        .iter()
        .find(|coach| coach.person == "Josh Stephens")
        .ok_or("missing athletic director")?;
    director
        .sport
        .is_none()
        .then_some(())
        .ok_or_else(|| format!("director sport: left={:?}, right=None", director.sport))?;
    if director.role != CoachRole::AthleticDirector {
        return Err(format!(
            "director role: left={:?}, right={:?}",
            director.role,
            CoachRole::AthleticDirector
        )
        .into());
    }
    Ok(())
}

#[test]
fn tssaa_detail_id407_keeps_ad_without_inventing_sport_appointments() -> TestResult {
    let id = StateRecordId::parse("407")?;
    let read = parse_school_page(DETAIL_407, &id)?;
    let entries = read.school.entries().len();
    if entries != 1 {
        return Err(format!("one school entry: left={entries}, right=1").into());
    }
    if let Some(coach) = read.coaches.iter().find(|coach| coach.sport.is_some()) {
        return Err(format!(
            "unexpected sport appointment: person={}, sport={:?}",
            coach.person, coach.sport
        )
        .into());
    }
    let director = read
        .coaches
        .iter()
        .find(|coach| coach.person == "Tiffany Ayers")
        .ok_or("missing athletic director")?;
    if director.role != CoachRole::AthleticDirector {
        return Err(format!(
            "director role: left={:?}, right={:?}",
            director.role,
            CoachRole::AthleticDirector
        )
        .into());
    }
    if director.gender != Gender::Unknown {
        return Err(format!(
            "director gender: left={:?}, right={:?}",
            director.gender,
            Gender::Unknown
        )
        .into());
    }
    let email = director.email.as_deref();
    if email != Some("tiffany@redemptionlife.net") {
        return Err(format!(
            "director email: left={email:?}, right={:?}",
            Some("tiffany@redemptionlife.net")
        )
        .into());
    }
    let phone = director.phone.as_deref();
    if phone != Some("865-440-0976") {
        return Err(format!(
            "director phone: left={phone:?}, right={:?}",
            Some("865-440-0976")
        )
        .into());
    }
    let entry = read.school.entries().first().ok_or("missing entry")?;
    let name = entry.name().map(SchoolName::as_str);
    if name != Some("Redemption School of Worship") {
        return Err(format!(
            "school name: left={name:?}, right={:?}",
            Some("Redemption School of Worship")
        )
        .into());
    }
    Ok(())
}
