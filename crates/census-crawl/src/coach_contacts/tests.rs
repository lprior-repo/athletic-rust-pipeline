use super::*;
use census_domain::model::{CanonicalCoach, CanonicalSchool, CoachRole, Gender, Sport};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

const CSV: &str = include_str!("../../tests/fixtures/coach_contacts_sample.csv");

fn rows() -> Vec<CoachContactRow> {
    let mut reader = csv::Reader::from_reader(CSV.as_bytes());
    reader
        .deserialize::<CoachContactRow>()
        .map(|row| row.unwrap())
        .collect()
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
    assert_eq!(
        parse_role("Activities Director"),
        Some(CoachRole::AthleticDirector)
    );
}

#[test]
fn coach_rows_become_canonical_entities_with_evidence() {
    let rows = rows();
    let wiaa = rows
        .iter()
        .find(|row| row.school == "Abbotsford")
        .expect("fixture row");
    let entities = row_entities(wiaa, UsJurisdiction::Wisconsin, "2026-09-20").unwrap();
    assert_eq!(entities.school.state, Some(UsJurisdiction::Wisconsin));
    assert_eq!(entities.school.city.as_deref(), Some("Abbotsford"));
    assert_eq!(entities.school.name, "Abbotsford");
    // one sport coach + one AD
    assert_eq!(entities.coaches.len(), 2);
    let coach = entities
        .coaches
        .iter()
        .find(|coach| coach.role == CoachRole::HeadCoach)
        .unwrap();
    assert_eq!(coach.name, "JACOB KNAPMILLER");
    assert_eq!(coach.sport, Some(Sport::OutdoorTrack));
    assert_eq!(coach.gender, Gender::Boys);
    assert_eq!(
        coach.professional_email.as_deref(),
        Some("jknapmiller@abbotsford.k12.wi.us")
    );
    let ad = entities
        .coaches
        .iter()
        .find(|coach| coach.role == CoachRole::AthleticDirector)
        .unwrap();
    // Athletic directors are school-wide: no sport binding.
    assert_eq!(ad.sport, None);
    assert_eq!(
        ad.professional_email.as_deref(),
        Some("alarson@abbotsford.k12.wi.us")
    );
    assert!(coach.evidence.iter().all(|evidence| evidence
        .source
        .url
        .as_deref()
        .unwrap()
        .contains("orgID=1")));
}

#[test]
fn import_dedupes_schools_and_ad_rows() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let path = dir.path().join("coach-contacts.csv");
    std::fs::write(&path, CSV).unwrap();
    let report = import_csv(&store, &path, "2026-09-20").unwrap();
    assert!(report.rows > 0, "coach rows imported: {}", report.rows);

    let read_schools = || -> Vec<serde_json::Value> {
        store
            .scan::<CanonicalSchool>(Table::Schools)
            .unwrap()
            .into_iter()
            .map(|v| serde_json::to_value(&v).unwrap())
            .collect()
    };
    let read_coaches = || -> Vec<serde_json::Value> {
        store
            .scan::<CanonicalCoach>(Table::Coaches)
            .unwrap()
            .into_iter()
            .map(|v| serde_json::to_value(&v).unwrap())
            .collect()
    };
    let schools = read_schools();
    let coaches = read_coaches();

    // Four Abbotsford rows collapse to one school record; the MI slug/name forms stay distinct
    // from each other but each is a single record.
    let abbotsford = schools
        .iter()
        .filter(|school| school["name"] == "Abbotsford")
        .count();
    assert_eq!(
        abbotsford, 1,
        "duplicate school rows collapse to one entity"
    );
    let school_names: Vec<String> = schools
        .iter()
        .map(|school| school["name"].as_str().unwrap().to_string())
        .collect();
    for expected in [
        "Abilene HS",
        "aberdeencentral",
        "Adams Central",
        "Abingdon-Avon High School",
    ] {
        assert!(
            school_names.iter().any(|name| name == expected),
            "missing school {expected} in {school_names:?}"
        );
    }

    // Non-coaching office staff are dropped even though they occupy the `ad_name` column.
    let coach_names: Vec<String> = coaches
        .iter()
        .map(|coach| coach["name"].as_str().unwrap().to_string())
        .collect();
    for dropped in ["Kevin Polston", "Omar Bakri", "Mindy Langlois"] {
        assert!(
            !coach_names.iter().any(|name| name == dropped),
            "non-coaching office staff imported: {dropped}"
        );
    }
    // The SD AD appears on two duplicate rows and must exist exactly once.
    assert_eq!(
        coach_names.iter().filter(|name| *name == "Bo Beck").count(),
        1,
        "duplicate AD rows collapse: {coach_names:?}"
    );
    // Honorifics are stripped so the IL coach matches a plain-name observation.
    assert!(
        coach_names.iter().any(|name| name == "Barry Mink"),
        "{coach_names:?}"
    );
    assert!(report.with_email > 0);
}
