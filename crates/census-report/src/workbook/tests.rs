use crate::csv_safety::protect_owned;
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalCoach, CanonicalMeet, CanonicalSchool,
    CoachContactClaim, CoachContactProgram, CoachRole, CoachTenure, CoachTenureEvidence, Evidence,
    Gender, GradYear, PublishedGraduation, SchoolYear, SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::path::Path;
use std::path::PathBuf;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const DAY: &str = "2026-09-20";
const FORMULA_ATHLETE: &str = "=cmd|' /C calc'!A0";
const FORMULA_SCHOOL: &str = "\t+1+1";
const FORMULA_CITY: &str = "\u{feff}@SUM(1)";
const FORMULA_COACH: &str = "-2+3";
const FORMULA_EMAIL: &str = "=2+2@example.test";
const FORMULA_MEET: &str = "=cmd|calc";

fn school(
    store: &Store,
    name: &str,
    city: Option<&str>,
) -> TestResult<census_domain::model::SchoolId> {
    let (mut school, id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name), city);
    school.athletics_website = Some("https://schools.test/athletics".to_string());
    school.evidence = vec![Evidence::parsed(
        SourceRef::new("fixture", Some("https://fixtures.test/schools".to_string())),
        DAY,
    )];
    store.append(Table::Schools, &school)?;
    Ok(id)
}

fn athlete(
    store: &Store,
    school: &census_domain::model::SchoolId,
    name: &str,
) -> TestResult<census_domain::model::AthleteId> {
    let mut athlete = CanonicalAthlete::new(
        school,
        name,
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::Other("fixture".to_owned()), "hostile"),
    );
    athlete.sports = vec![census_domain::model::Sport::OutdoorTrack];
    athlete.evidence = vec![Evidence::parsed(
        SourceRef::new("fixture", Some("https://fixtures.test/athlete".to_string())),
        DAY,
    )];
    athlete.published_graduations.push(PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new("fixture", Some("https://fixtures.test/athlete".to_string())),
    });
    store.append(Table::Athletes, &athlete)?;
    Ok(athlete.id)
}

fn coach(store: &Store, school: &census_domain::model::SchoolId) -> TestResult {
    let mut coach = CanonicalCoach::new(
        school,
        FORMULA_COACH,
        Some(census_domain::model::Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    coach.professional_email = Some(FORMULA_EMAIL.to_string());
    let claim = CoachContactClaim {
        coach: coach.id.clone(),
        school: school.clone(),
        role: coach.role,
        program: CoachContactProgram::Team {
            sport: census_domain::model::Sport::OutdoorTrack,
            gender: coach.gender,
        },
        mailbox: Some(FORMULA_EMAIL.to_string()),
    };
    let evidence = CoachTenureEvidence {
        tenure: CoachTenure::Current {
            school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
        },
        source: SourceRef::new("fixture", Some("https://contacts.test".to_string())),
        source_sha256: "a".repeat(64),
        retrieved_at: "2026-09-20T00:00:00Z".into(),
        statement: "Synthetic appointment".into(),
        claim: Some(claim),
    };
    coach.tenure_evidence = vec![evidence];
    store.append(Table::Coaches, &coach)?;
    Ok(())
}

fn column_of(range: &Range<Data>, header: &str) -> TestResult<usize> {
    (0..range.width())
        .find(|column| {
            range
                .get_value((0, *column as u32))
                .map_or(false, |value| value.to_string() == header)
        })
        .ok_or_else(|| format!("missing column {header}").into())
}

fn cell_text(range: &Range<Data>, row: usize, column: usize) -> String {
    range
        .get_value((row as u32, column as u32))
        .map(|value| value.to_string())
        .unwrap_or_default()
}

fn cell_string(range: &Range<Data>, row: usize, column: usize) -> Option<String> {
    match range.get((row, column)) {
        Some(Data::String(value)) => Some(value.as_str().to_string()),
        _ => None,
    }
}

fn verify_literal(range: &Range<Data>, row: usize, header: &str, expected: &str) -> TestResult {
    let column = column_of(range, header)?;
    match cell_string(range, row, column) {
        Some(value) if value == expected => Ok(()),
        Some(value) => {
            Err(format!("cell {header}: expected literal {expected:?}, got {value:?}").into())
        }
        None => Err(format!("cell {header} is not a literal string").into()),
    }
}

fn row_where(range: &Range<Data>, header: &str, value: &str) -> TestResult<usize> {
    let column = column_of(range, header)?;
    (1..range.height())
        .find(|row| cell_text(range, *row, column) == value)
        .ok_or_else(|| format!("missing row with {header}={value}").into())
}

fn build_hostile_workbook() -> TestResult<(Store, PathBuf)> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school_id = school(&store, FORMULA_SCHOOL, Some(FORMULA_CITY))?;
    athlete(&store, &school_id, FORMULA_ATHLETE)?;
    coach(&store, &school_id)?;
    let options = crate::workbook::Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: crate::report::Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
    };
    let published = crate::workbook::build(&store, &options)?;
    let _root = dir.keep();
    Ok((store, published))
}

#[test]
fn all_workbook_sheets_write_formula_prefixed_text_as_literal_strings() -> TestResult {
    let (_store, path) = build_hostile_workbook()?;

    let mut book: Xlsx<_> = open_workbook(&path)?;
    let names: Vec<String> = book.sheet_names().iter().map(|n| n.to_string()).collect();
    check!(names.contains(&"Athletes".to_string()));
    check!(names.contains(&"Coaches".to_string()));

    let athletes = book.worksheet_range("Athletes")?;
    let athlete_row = row_where(&athletes, "Name", FORMULA_ATHLETE)?;
    verify_literal(&athletes, athlete_row, "Name", FORMULA_ATHLETE)?;
    verify_literal(&athletes, athlete_row, "School", FORMULA_SCHOOL)?;
    verify_literal(&athletes, athlete_row, "School City", FORMULA_CITY)?;

    let coaches = book.worksheet_range("Coaches")?;
    let coach_row = row_where(&coaches, "Coach", FORMULA_COACH)?;
    verify_literal(&coaches, coach_row, "Coach", FORMULA_COACH)?;
    verify_literal(&coaches, coach_row, "Professional Email", FORMULA_EMAIL)?;

    Ok(())
}

fn seed_hostile_meet(store: &Store) -> TestResult {
    let meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        FORMULA_MEET,
        "2026-05-01",
        census_domain::model::CompetitionLevel::Invitational,
    );
    store.append(Table::Meets, &meet)?;
    Ok(())
}

#[test]
fn meta_sheets_write_formula_prefixed_text_as_literal_strings() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    school(&store, FORMULA_SCHOOL, Some(FORMULA_CITY))?;
    seed_hostile_meet(&store)?;

    let options = crate::workbook::Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: crate::report::Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
    };
    let path = crate::workbook::build(&store, &options)?;

    let mut book: Xlsx<_> = open_workbook(&path)?;
    let names: Vec<String> = book.sheet_names().iter().map(|n| n.to_string()).collect();
    check!(names.contains(&"Schools".to_string()));
    check!(names.contains(&"Meets".to_string()));

    let schools = book.worksheet_range("Schools")?;
    let row = row_where(&schools, "School", FORMULA_SCHOOL)?;
    verify_literal(&schools, row, "School", FORMULA_SCHOOL)?;
    verify_literal(&schools, row, "City", FORMULA_CITY)?;

    let meets = book.worksheet_range("Meets")?;
    let row = row_where(&meets, "Meet", FORMULA_MEET)?;
    verify_literal(&meets, row, "Meet", FORMULA_MEET)?;

    Ok(())
}

fn write_recruiting_csv(store: &Store, path: &Path) -> TestResult {
    let dataset = crate::export::ExportDataset::load(store)?;
    let derivation =
        crate::report::Derivation::of(&dataset, crate::report::Scope::AllSources, Some(2027));
    crate::workbook::write_recruiting_csv(
        &derivation,
        SchoolYear::new(2026).ok_or("invalid fixture season")?,
        path,
    )?;
    Ok(())
}

#[test]
fn recruiting_csv_protects_formula_prefixed_text() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school_id = school(&store, FORMULA_SCHOOL, Some(FORMULA_CITY))?;
    athlete(&store, &school_id, FORMULA_ATHLETE)?;
    coach(&store, &school_id)?;

    let csv_path = dir.path().join("recruiting.csv");
    write_recruiting_csv(&store, &csv_path)?;

    let mut reader = csv::ReaderBuilder::new().from_path(&csv_path)?;
    let headers = reader.headers()?.clone();
    let name_index = headers
        .iter()
        .position(|h| h == "name")
        .ok_or("missing name header")?;
    let school_index = headers
        .iter()
        .position(|h| h == "school")
        .ok_or("missing school header")?;

    let mut rows = 0usize;
    let mut hostile_rows = 0usize;
    for record in reader.records() {
        let record = record?;
        rows += 1;
        if record.iter().any(|field| field.contains(FORMULA_ATHLETE)) {
            hostile_rows += 1;
            check!(
                record[name_index].starts_with("'"),
                "CSV name field must be single-quoted to prevent formula evaluation"
            );
            check!(
                record[school_index].starts_with("'"),
                "CSV school field must be single-quoted to prevent formula evaluation"
            );
        }
    }
    check!(rows > 0);
    check!(
        hostile_rows > 0,
        "the hostile athlete must publish a CSV row to protect"
    );

    Ok(())
}

#[test]
fn csv_protection_adds_quote_only_when_needed() {
    assert_eq!(protect_owned("=cmd".to_string()).unwrap(), "'=cmd");
    assert_eq!(protect_owned("+cmd".to_string()).unwrap(), "'+cmd");
    assert_eq!(protect_owned("-cmd".to_string()).unwrap(), "'-cmd");
    assert_eq!(protect_owned("@cmd".to_string()).unwrap(), "'@cmd");
    assert_eq!(protect_owned("\t+cmd".to_string()).unwrap(), "'\t+cmd");
    assert_eq!(
        protect_owned("\u{feff}@cmd".to_string()).unwrap(),
        "'\u{feff}@cmd"
    );
    assert_eq!(
        protect_owned("normal text".to_string()).unwrap(),
        "normal text"
    );
    assert_eq!(protect_owned("123".to_string()).unwrap(), "123");
    assert_eq!(protect_owned("A&B".to_string()).unwrap(), "A&B");
}
