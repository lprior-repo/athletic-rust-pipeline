use super::map::{school_entities, ProfileFacts};
use super::parse::{parse_directory_links, parse_school_details, parse_sport_and_gender};
use census_domain::model::{CoachRole, Gender, Sport};
use census_domain::UsJurisdiction;

const DIRECTORY_FL: &str =
    include_str!("../../tests/fixtures/home_campus/directory_section_10.html");
const DIRECTORY_NJ: &str =
    include_str!("../../tests/fixtures/home_campus/directory_section_12.html");
const DETAILS_19: &str = include_str!("../../tests/fixtures/home_campus/details_19.json");
const DETAILS_1872: &str = include_str!("../../tests/fixtures/home_campus/details_1872.json");
const DETAILS_3374: &str = include_str!("../../tests/fixtures/home_campus/details_3374.json");

fn facts<'a>(
    state: UsJurisdiction,
    association: &'a str,
    name: &'a str,
    city: &'a str,
    url: &'a str,
) -> ProfileFacts<'a> {
    ProfileFacts {
        state,
        association,
        name,
        city,
        address: "",
        zip: "",
        league: "",
        phone: "",
        url,
        observed_on: "2026-10-04",
    }
}

#[test]
fn florida_directory_parses_every_school_button() {
    let links = parse_directory_links(DIRECTORY_FL);
    assert_eq!(links.len(), 880);
    assert_eq!(links[0].id, 2397);
    assert_eq!(links[0].name, "Abundant Life Christian (Margate)");
    assert_eq!(links[1].name, "Academy at the Lakes (Land O'Lakes)");
    let mut ids: Vec<u64> = links.iter().map(|link| link.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 880);
}

#[test]
fn new_jersey_directory_parses_every_school_button() {
    let links = parse_directory_links(DIRECTORY_NJ);
    assert_eq!(links.len(), 452);
    assert!(links.iter().all(|link| !link.name.is_empty()));
    let mut ids: Vec<u64> = links.iter().map(|link| link.id).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), 452);
}

#[test]
fn arcadia_maps_cross_country_track_and_the_athletic_director() {
    let Some(details) = parse_school_details(DETAILS_19) else {
        panic!("details_19.json must parse");
    };
    assert_eq!(details.profile.id, 19);
    assert_eq!(details.profile.name, "Arcadia");
    assert_eq!(details.profile.city, "Arcadia");
    assert_eq!(details.profile.zip, "91007");
    assert_eq!(details.coaches.len(), 24);
    assert_eq!(details.faculties.len(), 6);

    let extract = school_entities(
        &facts(
            UsJurisdiction::California,
            "cif",
            "Arcadia",
            "Arcadia",
            "https://www.cifsshome.org/widget/get-school-details/19/details",
        ),
        &details.coaches,
        &details.faculties,
    );

    assert_eq!(extract.school.state, Some(UsJurisdiction::California));
    assert_eq!(extract.school.city.as_deref(), Some("Arcadia"));
    assert_eq!(extract.school.association.as_deref(), Some("cif"));
    assert_eq!(extract.coaches.len(), 5);

    let sport_coaches: Vec<&census_domain::model::CanonicalCoach> = extract
        .coaches
        .iter()
        .filter(|coach| coach.sport.is_some())
        .collect();
    assert_eq!(sport_coaches.len(), 4);
    assert_eq!(
        sport_coaches
            .iter()
            .filter(|coach| coach.sport == Some(Sport::CrossCountry))
            .count(),
        2
    );
    assert_eq!(
        sport_coaches
            .iter()
            .filter(|coach| coach.sport == Some(Sport::OutdoorTrack))
            .count(),
        2
    );
    assert!(sport_coaches
        .iter()
        .all(|coach| coach.role == CoachRole::HeadCoach));
    assert!(sport_coaches
        .iter()
        .all(|coach| coach.gender != Gender::Unknown));

    let directors: Vec<&census_domain::model::CanonicalCoach> = extract
        .coaches
        .iter()
        .filter(|coach| coach.role == CoachRole::AthleticDirector)
        .collect();
    assert_eq!(directors.len(), 1);
    assert_eq!(directors[0].name, "Levi Sieg");
    assert_eq!(
        directors[0].professional_email.as_deref(),
        Some("lsieg@ausd.net")
    );
    assert_eq!(directors[0].sport, None);
    assert_eq!(directors[0].gender, Gender::Unknown);

    assert!(extract
        .coaches
        .iter()
        .all(|coach| coach.has_published_email()));
    assert!(extract
        .coaches
        .iter()
        .all(|coach| !coach.source_identities.is_empty()));
}

#[test]
fn bolles_skips_the_null_roster_row_and_keeps_published_contacts() {
    let Some(details) = parse_school_details(DETAILS_1872) else {
        panic!("details_1872.json must parse");
    };
    assert_eq!(details.profile.name, "Bolles (Jacksonville)");
    assert_eq!(details.coaches.len(), 26);
    assert_eq!(details.faculties.len(), 8);

    let extract = school_entities(
        &facts(
            UsJurisdiction::Florida,
            "fhsaa",
            "Bolles (Jacksonville)",
            "Jacksonville",
            "https://www.cifsshome.org/widget/get-school-details/1872/details",
        ),
        &details.coaches,
        &details.faculties,
    );

    assert_eq!(extract.school.state, Some(UsJurisdiction::Florida));
    assert_eq!(
        extract
            .coaches
            .iter()
            .filter(|coach| coach.sport.is_some())
            .count(),
        3
    );
    let directors: Vec<&census_domain::model::CanonicalCoach> = extract
        .coaches
        .iter()
        .filter(|coach| coach.role == CoachRole::AthleticDirector)
        .collect();
    assert_eq!(directors.len(), 1);
    assert_eq!(directors[0].name, "Rock Pillsbury");
    assert_eq!(
        directors[0].professional_email.as_deref(),
        Some("pillsburyr@bolles.org")
    );
}

#[test]
fn school_without_published_roster_maps_to_no_coaches() {
    let Some(details) = parse_school_details(DETAILS_3374) else {
        panic!("details_3374.json must parse");
    };
    assert_eq!(details.profile.name, "Abraham Clark");
    assert!(details.coaches.is_empty());
    assert!(details.faculties.is_empty());

    let extract = school_entities(
        &facts(
            UsJurisdiction::NewJersey,
            "njsiaa",
            "Abraham Clark",
            "",
            "https://www.cifsshome.org/widget/get-school-details/3374/details",
        ),
        &details.coaches,
        &details.faculties,
    );

    assert_eq!(extract.school.state, Some(UsJurisdiction::NewJersey));
    assert_eq!(extract.school.association.as_deref(), Some("njsiaa"));
    assert!(extract.coaches.is_empty());
}

#[test]
fn only_cross_country_and_track_field_labels_map_to_sports() {
    assert_eq!(
        parse_sport_and_gender("Cross Country, Boys"),
        Some((Sport::CrossCountry, Gender::Boys))
    );
    assert_eq!(
        parse_sport_and_gender("Cross Country, Girls"),
        Some((Sport::CrossCountry, Gender::Girls))
    );
    assert_eq!(
        parse_sport_and_gender("Track & Field, Boys"),
        Some((Sport::OutdoorTrack, Gender::Boys))
    );
    assert_eq!(
        parse_sport_and_gender("Track and Field, Girls"),
        Some((Sport::OutdoorTrack, Gender::Girls))
    );
    assert_eq!(parse_sport_and_gender("Badminton"), None);
    assert_eq!(parse_sport_and_gender("Soccer, Girls"), None);
    assert_eq!(parse_sport_and_gender(""), None);
}
