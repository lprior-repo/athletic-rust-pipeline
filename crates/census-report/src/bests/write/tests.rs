use super::write;
use crate::bests::{
    ComparisonPolicy, Measure, Population, PrKey, SelectionAthlete, SelectionMeet, SelectionResult,
    SelectionSource, SharedSelection, SurfaceClass, TimingClass, WindClass,
};
use census_domain::model::{
    EventKind, EventSpecification, ExactSeconds, Gender, Id, Mark, SourceRef, TimingMethod,
};
use census_domain::{JurisdictionBucket, MeetState, UsJurisdiction};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

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
        key: key(),
        result: result(mark, wind_mps),
        meet: selection_meet(meet),
        source: source(athlete),
        athlete: selection_athlete(athlete, school),
        population: Population {
            marks: 3,
            sources: 2,
        },
        conflicts: Vec::new(),
        eligible_candidate_count: 3,
        tied_performance_count: 1,
        tied_performance_ids: vec![Id::mint("perf", &["csv_safety"])],
        supporting_tied_candidate_ids: vec![Id::mint("perf", &["csv_safety"])],
    }
}

fn key() -> PrKey {
    PrKey {
        athlete_id: Id::mint("ath", &["csv_safety"]),
        event_kind: EventKind::Track100m,
        surface: SurfaceClass::Outdoor,
        wind_class: WindClass::Legal,
        timing: TimingClass::Fat,
        measure: Measure::Time,
        context: None,
        specification: EventSpecification::default(),
        comparison: ComparisonPolicy::Standard,
        adjustment: None,
    }
}

fn result(mark: Mark, wind_mps: Option<f64>) -> SelectionResult {
    SelectionResult {
        value: 10_940_000_000,
        normalized: Some(10.94),
        mark,
        place: Some(3),
        wind_mps,
        timing: Some(TimingMethod::Fat),
        original_mark: None,
        adjustment_kind: None,
        adjustment_basis: None,
        adjustment_note: None,
        adjustment_source: None,
    }
}

fn selection_meet(meet: &str) -> SelectionMeet {
    SelectionMeet {
        date: "2025-03-01".to_string(),
        name: meet.to_string(),
        meet_id: Id::mint("meet", &["csv_safety"]),
        meet_state: MeetState::Placed(UsJurisdiction::Wisconsin),
        heat: None,
        round: None,
    }
}

fn source(athlete: &str) -> SelectionSource {
    SelectionSource {
        specification: EventSpecification::default(),
        result_url: "https://results.test/meet".to_string(),
        performance_id: Id::mint("perf", &["csv_safety"]),
        source_athlete: athlete.to_string(),
        source_key: "wiaa_results:2025-03-01:time".to_string(),
    }
}

fn selection_athlete(athlete: &str, school: &str) -> SelectionAthlete {
    SelectionAthlete {
        name: athlete.to_string(),
        gender: Gender::Boys,
        grad_year: 2027,
        profile_url: Some("https://profiles.test/athlete".to_string()),
        school: Some(school.to_string()),
        athlete_school: school.to_string(),
        athlete_state: JurisdictionBucket::Jurisdiction(UsJurisdiction::Wisconsin),
    }
}

fn column(header: &csv::StringRecord, name: &str) -> TestResult<usize> {
    header
        .iter()
        .position(|field| field == name)
        .ok_or_else(|| format!("missing published column {name}").into())
}

#[test]
fn csv_text_is_literalized_while_the_raw_jsonl_is_preserved() -> TestResult {
    let directory = tempfile::tempdir()?;
    let rows = [selection(
        FORMULA_NAME,
        FORMULA_SCHOOL,
        FORMULA_MEET,
        Mark::Raw(FORMULA_MARK.to_string()),
        Some(-1.4),
    )];
    let (jsonl, csv) = write(directory.path(), &rows, "co2027")?;

    let mut reader = csv::Reader::from_path(&csv)?;
    let header = reader.headers()?.clone();
    let row = reader.records().next().ok_or("missing CSV data row")??;
    for (name, value) in [
        ("name", FORMULA_NAME),
        ("school", FORMULA_SCHOOL),
        ("meet", FORMULA_MEET),
        ("best_mark", FORMULA_MARK),
    ] {
        check!(eq; row.get(column(&header, name)?),
        Some(format!("'{value}").as_str()),
        "{name}");
    }
    check!(eq; row.get(column(&header, "wind_mps")?), Some("-1.4"));
    check!(eq; row.get(column(&header, "place")?), Some("3"));

    let jsonl_text = std::fs::read_to_string(&jsonl)?;
    for value in [FORMULA_NAME, FORMULA_SCHOOL, FORMULA_MEET, FORMULA_MARK] {
        check!(jsonl_text.contains(value), "the JSONL keeps {value}");
        check!(
            !jsonl_text.contains(&format!("'{value}")),
            "the JSONL is not literalized"
        );
    }
    Ok(())
}

#[test]
fn ordinary_text_and_negative_numerics_publish_unchanged() -> TestResult {
    let directory = tempfile::tempdir()?;
    let rows = [selection(
        ORDINARY_NAME,
        "Ordinary School",
        "Ordinary Meet",
        Mark::TimeSeconds(ExactSeconds::parse("10.94")?),
        Some(-2.3),
    )];
    let (_, csv) = write(directory.path(), &rows, "co2027")?;

    let text = std::fs::read_to_string(&csv)?;
    check!(text.contains("Ordinary Runner,Ordinary School"));
    check!(text.contains(",-2.3,"));
    check!(!text.contains("'-2.3"));

    let mut reader = csv::Reader::from_path(&csv)?;
    let header = reader.headers()?.clone();
    let row = reader.records().next().ok_or("missing CSV data row")??;
    check!(eq; row.get(column(&header, "name")?), Some(ORDINARY_NAME));
    check!(eq; row.get(column(&header, "wind_mps")?), Some("-2.3"));
    check!(eq; row.get(column(&header, "best_mark")?), Some("10.94"));
    check!(eq; row.get(column(&header, "best_value")?), Some("10940000000"));
    check!(eq; row.get(column(&header, "best_value_unit")?), Some("ns"));
    check!(eq; row.get(column(&header, "grad_year")?), Some("2027"));
    Ok(())
}

#[test]
fn converted_marks_publish_note_and_source_beside_kind_and_basis() -> TestResult {
    use census_domain::model::{AdjustmentBasis, AdjustmentKind};
    let directory = tempfile::tempdir()?;
    let mut converted = selection(
        ORDINARY_NAME,
        "Ordinary School",
        "Ordinary Meet",
        Mark::TimeSeconds(ExactSeconds::parse("10.94")?),
        None,
    );
    converted.result.original_mark = Some(Mark::TimeSeconds(ExactSeconds::parse("11.00")?));
    converted.result.adjustment_kind = Some(AdjustmentKind::TrackSizeConversion);
    converted.result.adjustment_basis = Some(AdjustmentBasis::Known);
    converted.result.adjustment_note = Some("Converted from 11.00 for Track Size. ".to_string());
    converted.result.adjustment_source = Some(SourceRef::new(
        "tfrrs_in",
        Some("https://indiana.tfrrs.org".to_string()),
    ));
    let (jsonl, csv) = write(directory.path(), &[converted], "co2027")?;
    let mut reader = csv::Reader::from_path(&csv)?;
    let header = reader.headers()?.clone();
    let row = reader.records().next().ok_or("missing CSV data row")??;
    check!(eq; row.get(column(&header, "original_mark")?), Some("11.00"));
    check!(eq; row.get(column(&header, "adjustment_kind")?), Some("track_size_conversion"));
    check!(eq; row.get(column(&header, "adjustment_basis")?), Some("known"));
    check!(
        eq;
        row.get(column(&header, "adjustment_note")?),
        Some("Converted from 11.00 for Track Size. ")
    );
    let source = row
        .get(column(&header, "adjustment_source")?)
        .ok_or("missing source")?;
    check!(source.contains("tfrrs_in"), "the source id survives");
    check!(
        source.contains("indiana.tfrrs.org"),
        "the source url survives"
    );
    let jsonl_text = std::fs::read_to_string(&jsonl)?;
    check!(jsonl_text.contains("Converted from 11.00 for Track Size. "));
    check!(jsonl_text.contains("tfrrs_in"));
    Ok(())
}
