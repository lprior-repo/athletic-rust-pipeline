use super::write;
use crate::bests::{
    Measure, Population, PrKey, SharedSelection, SurfaceClass, TimingClass, WindClass,
};
use census_domain::model::{CentiSeconds, EventKind, Gender, Id, Mark, TimingMethod};
use census_domain::{JurisdictionBucket, MeetState, UsJurisdiction};

const FORMULA_MARK: &str = "=cmd|' /C calc'!A0";
const FORMULA_NAME: &str = "=SUM(1+1)";
const FORMULA_SCHOOL: &str = "@SUM(1)";
const FORMULA_MEET: &str = "+Invitational";
const ORDINARY_NAME: &str = "Ordinary Runner";

fn selection(
    athlete: &str,
    school: &str,
    meet: &str,
    mark: Mark,
    wind_mps: Option<f64>,
) -> SharedSelection {
    SharedSelection {
        key: PrKey {
            athlete_id: Id::mint("ath", &["csv_safety"]),
            event_kind: EventKind::Track100m,
            surface: SurfaceClass::Outdoor,
            wind_class: WindClass::Legal,
            timing: TimingClass::Fat,
            measure: Measure::Time,
            context: None,
        },
        value: 1094,
        normalized: Some(10.94),
        mark,
        date: "2025-03-01".to_string(),
        meet: meet.to_string(),
        meet_id: Id::mint("meet", &["csv_safety"]),
        meet_state: MeetState::Placed(UsJurisdiction::Wisconsin),
        place: Some(3),
        wind_mps,
        timing: Some(TimingMethod::Fat),
        result_url: "https://results.test/meet".to_string(),
        performance_id: Id::mint("perf", &["csv_safety"]),
        source_athlete: athlete.to_string(),
        source_key: "wiaa_results:2025-03-01:time".to_string(),
        athlete: athlete.to_string(),
        gender: Gender::Boys,
        grad_year: 2027,
        profile_url: Some("https://profiles.test/athlete".to_string()),
        school: Some(school.to_string()),
        athlete_school: school.to_string(),
        athlete_state: JurisdictionBucket::Jurisdiction(UsJurisdiction::Wisconsin),
        population: Population {
            marks: 3,
            sources: 2,
        },
        conflicts: Vec::new(),
    }
}

fn column(header: &csv::StringRecord, name: &str) -> usize {
    header
        .iter()
        .position(|field| field == name)
        .expect("published column")
}

#[test]
fn csv_text_is_literalized_while_the_raw_jsonl_is_preserved() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let rows = [selection(
        FORMULA_NAME,
        FORMULA_SCHOOL,
        FORMULA_MEET,
        Mark::Raw(FORMULA_MARK.to_string()),
        Some(-1.4),
    )];
    let (jsonl, csv) = write(directory.path(), &rows, "co2027").expect("snapshot publishes");

    let mut reader = csv::Reader::from_path(&csv).expect("published CSV reads");
    let header = reader.headers().expect("header row").clone();
    let row = reader
        .records()
        .next()
        .expect("one data row")
        .expect("data row decodes");
    for (name, value) in [
        ("name", FORMULA_NAME),
        ("school", FORMULA_SCHOOL),
        ("meet", FORMULA_MEET),
        ("best_mark", FORMULA_MARK),
    ] {
        assert_eq!(
            row.get(column(&header, name)),
            Some(format!("'{value}").as_str()),
            "{name}"
        );
    }
    assert_eq!(row.get(column(&header, "wind_mps")), Some("-1.4"));
    assert_eq!(row.get(column(&header, "place")), Some("3"));

    let jsonl_text = std::fs::read_to_string(&jsonl).expect("JSONL reads");
    for value in [FORMULA_NAME, FORMULA_SCHOOL, FORMULA_MEET, FORMULA_MARK] {
        assert!(jsonl_text.contains(value), "the JSONL keeps {value}");
        assert!(
            !jsonl_text.contains(&format!("'{value}")),
            "the JSONL is not literalized"
        );
    }
}

#[test]
fn ordinary_text_and_negative_numerics_publish_unchanged() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let rows = [selection(
        ORDINARY_NAME,
        "Ordinary School",
        "Ordinary Meet",
        Mark::TimeSeconds(CentiSeconds::new(1094)),
        Some(-2.3),
    )];
    let (_, csv) = write(directory.path(), &rows, "co2027").expect("snapshot publishes");

    let text = std::fs::read_to_string(&csv).expect("published CSV reads");
    assert!(text.contains("Ordinary Runner,Ordinary School"));
    assert!(text.contains(",-2.3,"));
    assert!(!text.contains("'-2.3"));

    let mut reader = csv::Reader::from_path(&csv).expect("published CSV reads");
    let header = reader.headers().expect("header row").clone();
    let row = reader
        .records()
        .next()
        .expect("one data row")
        .expect("data row decodes");
    assert_eq!(row.get(column(&header, "name")), Some(ORDINARY_NAME));
    assert_eq!(row.get(column(&header, "wind_mps")), Some("-2.3"));
    assert_eq!(row.get(column(&header, "best_mark")), Some("10.94"));
    assert_eq!(row.get(column(&header, "best_value")), Some("1094"));
    assert_eq!(row.get(column(&header, "grad_year")), Some("2027"));
}
