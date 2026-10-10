use super::entities::row_entities;
use super::wire::CoachContactRow;
use super::*;
use census_domain::model::{CanonicalCoach, CanonicalSchool, CoachRole, Gender, Sport};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CSV: &str = include_str!("../../tests/fixtures/coach_contacts_sample.csv");

fn rows() -> TestResult<Vec<CoachContactRow>> {
    let mut reader = csv::Reader::from_reader(CSV.as_bytes());
    Ok(reader
        .deserialize::<CoachContactRow>()
        .collect::<Result<_, _>>()?)
}

#[test]
fn parses_sport_and_gender_labels() {
    assert_eq!(
        parse_sport("Boys Track and Field"),
        Some((Sport::OutdoorTrack, Gender::Boys))
    );
    assert_eq!(
        parse_sport("Varsity Head Coach - Girls Cross Country"),
        Some((Sport::CrossCountry, Gender::Girls))
    );
    assert_eq!(
        parse_sport("Girls Track & Field Head Coach"),
        Some((Sport::OutdoorTrack, Gender::Girls))
    );
    assert_eq!(parse_sport(""), None);
}

#[test]
fn non_coaching_roles_are_not_imported() {
    assert_eq!(parse_role("Athletic Director Secretary"), None);
    assert_eq!(parse_role("Principal"), None);
    assert_eq!(parse_role("Superintendent"), None);
    assert_eq!(parse_role("Athletic Trainer"), None);
    assert_eq!(
        parse_role("Varsity Head Coach - Boys Cross Country"),
        Some(CoachRole::HeadCoach)
    );
    assert_eq!(
        parse_role("Varsity Assistant Coach - Girls Track & Field"),
        Some(CoachRole::AssistantCoach)
    );
    assert_eq!(parse_role("Activities Director"), None);
}

#[test]
fn listed_titles_map_to_the_run_role_vocabulary() {
    assert_eq!(parse_role("Varsity Head Coach"), Some(CoachRole::HeadCoach));
    assert_eq!(parse_role("Co-Head Coach"), Some(CoachRole::HeadCoach));
    assert_eq!(parse_role("Girls Head Coach"), Some(CoachRole::HeadCoach));
    assert_eq!(parse_role("Head coach"), Some(CoachRole::HeadCoach));
    assert_eq!(parse_role("Head Coach "), Some(CoachRole::HeadCoach));
    assert_eq!(
        parse_role("Assistant Varsity Coach"),
        Some(CoachRole::AssistantCoach)
    );
    assert_eq!(
        parse_role("Varsity Assistant Coach"),
        Some(CoachRole::AssistantCoach)
    );
    assert_eq!(parse_role("Assistant"), Some(CoachRole::AssistantCoach));
    assert_eq!(
        parse_role("Varsity Assistant"),
        Some(CoachRole::AssistantCoach)
    );
    assert_eq!(parse_role("Throws Coach"), Some(CoachRole::AssistantCoach));
    assert_eq!(
        parse_role("Distance Coach"),
        Some(CoachRole::AssistantCoach)
    );
    assert_eq!(
        parse_role("Pole Vault Coach"),
        Some(CoachRole::AssistantCoach)
    );
    assert_eq!(
        parse_role("High Jump Coach"),
        Some(CoachRole::AssistantCoach)
    );
    assert_eq!(parse_role("Jumps Coach"), Some(CoachRole::AssistantCoach));
    assert_eq!(parse_role("Distance"), Some(CoachRole::AssistantCoach));
    assert_eq!(parse_role("Sprints"), Some(CoachRole::AssistantCoach));
    assert_eq!(
        parse_role("Director of Athletics"),
        Some(CoachRole::AthleticDirector)
    );
    assert_eq!(
        parse_role("Athletics Director"),
        Some(CoachRole::AthleticDirector)
    );
    assert_eq!(parse_role("Assistant Athletic Director"), None);
    assert_eq!(parse_role("District AD"), None);
    assert_eq!(parse_role("Athletic Admin"), None);
    assert_eq!(parse_role("Coach"), Some(CoachRole::Unknown));
    assert_eq!(parse_role("Volunteer Coach"), Some(CoachRole::Unknown));
    assert_eq!(parse_role("MS Coach"), Some(CoachRole::Unknown));
    assert_eq!(parse_role("Other"), None);
    assert_eq!(parse_role("Medical Official"), None);
}

#[test]
fn coach_rows_become_canonical_entities_with_evidence() -> TestResult {
    let rows = rows()?;
    let wiaa = rows
        .iter()
        .find(|row| row.school == "Abbotsford")
        .ok_or("fixture row")?;
    let entities = row_entities(wiaa, UsJurisdiction::Wisconsin, "2026-09-20")?;
    check!(eq; entities.school.state, Some(UsJurisdiction::Wisconsin));
    check!(eq; entities.school.city.as_deref(), Some("Abbotsford"));
    check!(eq; entities.school.name, "Abbotsford");
    check!(eq; entities.coaches.len(), 2);
    let coach = entities
        .coaches
        .iter()
        .find(|coach| coach.role == CoachRole::HeadCoach)
        .ok_or("head coach")?;
    check!(eq; coach.name, "JACOB KNAPMILLER");
    check!(eq; coach.sport, Some(Sport::OutdoorTrack));
    check!(eq; coach.gender, Gender::Boys);
    check!(eq;
        coach.professional_email.as_deref(),
        Some("jknapmiller@abbotsford.k12.wi.us")
    );
    let ad = entities
        .coaches
        .iter()
        .find(|coach| coach.role == CoachRole::AthleticDirector)
        .ok_or("athletic director")?;
    check!(eq; ad.sport, None);
    check!(eq;
        ad.professional_email.as_deref(),
        Some("alarson@abbotsford.k12.wi.us")
    );
    check!(coach.evidence.iter().all(|evidence| evidence
        .source
        .url
        .as_deref()
        .is_some_and(|url| url.contains("orgID=1"))));
    Ok(())
}

#[test]
fn import_preserves_published_people_and_excludes_non_coaching_roles() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let path = dir.path().join("coach-contacts.csv");
    std::fs::write(&path, CSV)?;
    import_csv(&store, &path, "2026-09-20")?;

    let read_schools = || -> TestResult<Vec<serde_json::Value>> {
        Ok(store
            .scan::<CanonicalSchool>(Table::Schools)?
            .into_iter()
            .map(|v| serde_json::to_value(&v))
            .collect::<Result<_, _>>()?)
    };
    let read_coaches = || -> TestResult<Vec<serde_json::Value>> {
        Ok(store
            .scan::<CanonicalCoach>(Table::Coaches)?
            .into_iter()
            .map(|v| serde_json::to_value(&v))
            .collect::<Result<_, _>>()?)
    };
    let schools = read_schools()?;
    let coaches = read_coaches()?;

    let school_names: Vec<String> = schools
        .iter()
        .map(|school| Ok(school["name"].as_str().ok_or("school name")?.to_string()))
        .collect::<TestResult<_>>()?;
    for expected in [
        "Abilene HS",
        "aberdeencentral",
        "Adams Central",
        "Abingdon-Avon High School",
    ] {
        check!(
            school_names.iter().any(|name| name == expected),
            "missing school {expected} in {school_names:?}"
        );
    }

    let coach_names: Vec<String> = coaches
        .iter()
        .map(|coach| Ok(coach["name"].as_str().ok_or("coach name")?.to_string()))
        .collect::<TestResult<_>>()?;
    for dropped in ["Kevin Polston", "Omar Bakri", "Mindy Langlois"] {
        check!(
            !coach_names.iter().any(|name| name == dropped),
            "non-coaching office staff imported: {dropped}"
        );
    }
    check!(
        coach_names.iter().any(|name| name == "Barry Mink"),
        "{coach_names:?}"
    );
    Ok(())
}
