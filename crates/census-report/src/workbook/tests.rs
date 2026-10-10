use crate::csv_safety::protect_owned;
use crate::workbook::cells::{Cell, SheetWriter};
use crate::workbook::{PerformanceProjection, PerformanceRow, ProjectedValue};
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, CoachContactClaim, CoachContactProgram,
    CoachRole, CoachTenure, CoachTenureEvidence, CompetitionLevel, EventIdentity, EventKind,
    EventSpecification, Evidence, ExactSeconds, Gender, GradYear, Mark, PublishedGraduation,
    SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use rust_xlsxwriter::Workbook;
use std::path::Path;

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
        SourceRef::new("fixture", Some("https://fixtures.test/schools".into())),
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
    let claim = PublishedGraduation {
        grad_year: GradYear::CO2027,
        source: SourceRef::new("fixture", Some("https://fixtures.test/athlete/2027".into())),
    };
    let claim_evidence = Evidence::parsed(claim.source.clone(), DAY);
    athlete.evidence = vec![claim_evidence];
    athlete.published_graduations.push(claim);
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
        source: SourceRef::new("fixture", Some("https://contacts.test".into())),
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
            matches!(
                range.get_value((0, *column as u32)),
                Some(Data::String(value)) if value.as_str() == header
            )
        })
        .ok_or_else(|| format!("missing column {header}").into())
}

fn cell_text(range: &Range<Data>, row: usize, column: usize) -> String {
    range
        .get_value((row as u32, column as u32))
        .map_or_else(String::new, |value| value.to_string())
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

fn build_hostile_workbook() -> TestResult<(Store, tempfile::TempDir)> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school_id = school(&store, FORMULA_SCHOOL, Some(FORMULA_CITY))?;
    athlete(&store, &school_id, FORMULA_ATHLETE)?;
    coach(&store, &school_id)?;
    crate::export::ExportDataset::load(&store)?;
    let options = crate::workbook::Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: crate::report::Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
    };
    crate::workbook::build(&store, &options)?;
    Ok((store, dir))
}

#[test]
fn all_workbook_sheets_write_formula_prefixed_text_as_literal_strings() -> TestResult {
    let (store, _dir) = build_hostile_workbook()?;
    let path =
        crate::workbook::publication::current_workbook(&store.out_dir().join("publication"))?;

    let mut book: Xlsx<_> = open_workbook(&path)?;
    let names: Vec<String> = book.sheet_names().iter().map(|n| n.to_string()).collect();
    check!(names.contains(&"Athletes".to_string()));
    check!(names.contains(&"Coaches".to_string()));

    let athletes = book.worksheet_range("Athletes")?;
    column_of(&athletes, "School")?;
    let athlete_row = row_where(&athletes, "Name", FORMULA_ATHLETE)?;
    verify_literal(&athletes, athlete_row, "Name", FORMULA_ATHLETE)?;
    verify_literal(&athletes, athlete_row, "School", FORMULA_SCHOOL)?;
    verify_literal(&athletes, athlete_row, "School City", FORMULA_CITY)?;

    let coaches = book.worksheet_range("Coaches")?;
    column_of(&coaches, "Coach")?;
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

    crate::export::ExportDataset::load(&store)?;
    let options = crate::workbook::Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: crate::report::Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
    };
    crate::workbook::build(&store, &options)?;

    let path =
        crate::workbook::publication::current_workbook(&store.out_dir().join("publication"))?;

    let mut book: Xlsx<_> = open_workbook(&path)?;
    let names: Vec<String> = book.sheet_names().iter().map(|n| n.to_string()).collect();
    check!(names.contains(&"Schools".to_string()));
    check!(names.contains(&"Meets".to_string()));

    let schools = book.worksheet_range("Schools")?;
    column_of(&schools, "School")?;
    let row = row_where(&schools, "School", FORMULA_SCHOOL)?;
    verify_literal(&schools, row, "School", FORMULA_SCHOOL)?;
    verify_literal(&schools, row, "City", FORMULA_CITY)?;

    let meets = book.worksheet_range("Meets")?;
    column_of(&meets, "Meet")?;
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

    let mut reader = csv::Reader::from_path(&csv_path)?;
    let headers = reader.headers()?.clone();
    check!(headers.iter().any(|h| h == "name"));
    check!(headers.iter().any(|h| h == "school"));

    let name_index = headers
        .iter()
        .position(|h| h == "name")
        .ok_or("missing name header")?;
    let school_index = headers
        .iter()
        .position(|h| h == "school")
        .ok_or("missing school header")?;

    let mut matched = false;
    for record in reader.records() {
        let record = record?;
        check!(record.len() == headers.len());
        if record.iter().any(|field| field.contains(FORMULA_ATHLETE)) {
            matched = true;
            let name_field = &record[name_index];
            let school_field = &record[school_index];
            check!(
                name_field.starts_with("'"),
                "CSV name field must be single-quoted to prevent formula evaluation"
            );
            check!(
                school_field.starts_with("'"),
                "CSV school field must be single-quoted to prevent formula evaluation"
            );
        }
    }
    check!(matched, "expected a CSV row for the hostile athlete");

    Ok(())
}

#[test]
fn csv_protection_adds_quote_only_when_needed() -> TestResult {
    check!(eq; protect_owned("=cmd".to_string())?, "'=cmd");
    check!(eq; protect_owned("+cmd".to_string())?, "'+cmd");
    check!(eq; protect_owned("-cmd".to_string())?, "'-cmd");
    check!(eq; protect_owned("@cmd".to_string())?, "'@cmd");
    check!(eq; protect_owned("\t+cmd".to_string())?, "'\t+cmd");
    check!(eq; protect_owned("\u{feff}@cmd".to_string())?, "'\u{feff}@cmd");
    check!(eq; protect_owned("normal text".to_string())?, "normal text");
    check!(eq; protect_owned("123".to_string())?, "123");
    check!(eq; protect_owned("A&B".to_string())?, "A&B");
    Ok(())
}

struct PartitionFixture {
    school: &'static str,
    athlete: &'static str,
    source_id: &'static str,
    date: &'static str,
    seconds: &'static str,
}

fn partition_fixtures() -> Vec<PartitionFixture> {
    vec![
        PartitionFixture {
            school: "Abbotsford",
            athlete: "Ada",
            source_id: "runner-a",
            date: "2026-05-01",
            seconds: "48.55",
        },
        PartitionFixture {
            school: "Abbotsford",
            athlete: "Ada",
            source_id: "runner-a",
            date: "2026-05-08",
            seconds: "48.10",
        },
        PartitionFixture {
            school: "Abbotsford",
            athlete: "Bo",
            source_id: "runner-b",
            date: "2026-05-08",
            seconds: "51.20",
        },
        PartitionFixture {
            school: "Abbotsford",
            athlete: "Bo",
            source_id: "runner-b",
            date: "2026-05-15",
            seconds: "50.90",
        },
        PartitionFixture {
            school: "Colby",
            athlete: "Cy",
            source_id: "runner-c",
            date: "2026-04-30",
            seconds: "49.75",
        },
        PartitionFixture {
            school: "Colby",
            athlete: "Dee",
            source_id: "runner-d",
            date: "2026-05-08",
            seconds: "52.05",
        },
        PartitionFixture {
            school: "Colby",
            athlete: "Dee",
            source_id: "runner-d",
            date: "2026-05-15",
            seconds: "51.60",
        },
    ]
}

fn seed_partition_store(store: &Store, fixture: &PartitionFixture) -> TestResult {
    let (mut school_row, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        fixture.school,
        normalize_name(fixture.school),
        None,
    );
    school_row
        .evidence
        .push(Evidence::parsed(SourceRef::new("wiaa_results", None), DAY));
    store.append(Table::Schools, &school_row)?;
    let mut athlete = CanonicalAthlete::new(
        &school_id,
        fixture.athlete,
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(
            SourceNamespace::Other("fixture".to_owned()),
            fixture.source_id,
        ),
    );
    athlete
        .evidence
        .push(Evidence::parsed(SourceRef::new("wiaa_results", None), DAY));
    store.append(Table::Athletes, &athlete)?;
    let meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "Partition Invite",
        fixture.date,
        CompetitionLevel::Invitational,
    );
    store.append(Table::Meets, &meet)?;
    let event = CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Track400m,
            gender: Gender::Boys,
            division: None,
            round: None,
        },
        EventSpecification::default(),
    )?;
    store.append(Table::Events, &event)?;
    let team = CanonicalTeam {
        id: CanonicalTeam::mint(
            &school_id,
            Sport::OutdoorTrack,
            Gender::Boys,
            SchoolYear::new(2026).ok_or("invalid fixture season")?,
        ),
        school: school_id.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Boys,
        school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
        level: None,
        source_identities: Vec::new(),
        evidence: vec![Evidence::parsed(SourceRef::new("wiaa_results", None), DAY)],
        retained_conflicts: Vec::new(),
    };
    store.append(Table::Teams, &team)?;
    let source_key = format!("partition:{}:{}", fixture.athlete, fixture.date);
    let performance = CanonicalPerformance {
        id: CanonicalPerformance::mint(&athlete.id, &meet.id, &event.id, fixture.date, &source_key),
        athlete: athlete.id.clone(),
        team: team.id.clone(),
        event: event.id.clone(),
        meet: meet.id.clone(),
        date: fixture.date.to_string(),
        mark: Mark::TimeSeconds(ExactSeconds::parse(fixture.seconds)?),
        wind_mps: None,
        place: None,
        heat: None,
        round: None,
        timing: None,
        observed_grade: None,
        evidence: vec![Evidence::parsed(SourceRef::new("wiaa_results", None), DAY)],
        source_key,
        source_athlete: athlete.source.clone(),
        retained_conflicts: Vec::new(),
    };
    store.append(Table::Performances, &performance)?;
    Ok(())
}

fn projected_partition_rows(store: &Store) -> TestResult<Vec<PerformanceRow>> {
    let dataset = crate::export::ExportDataset::load(store)?;
    let derivation =
        crate::report::Derivation::of(&dataset, crate::report::Scope::AllSources, None);
    let projection = PerformanceProjection::of(&derivation);
    Ok(derivation
        .performances()
        .iter()
        .map(|performance| projection.row(performance))
        .collect())
}

fn cells_of(row: &PerformanceRow) -> Vec<Cell> {
    row.values()
        .into_iter()
        .map(|value| match value {
            ProjectedValue::Text(text) => Cell::text(text),
            ProjectedValue::Number(Some(number)) => Cell::Number(number),
            ProjectedValue::Number(None) => Cell::Empty,
        })
        .collect()
}

const PARTITION_HEADER: [&str; 20] = [
    "Canonical Result ID",
    "Athlete ID",
    "Athlete",
    "School",
    "Graduation Year",
    "Meet ID",
    "Meet",
    "Date",
    "State",
    "Sport",
    "Event",
    "Mark",
    "Normalized Mark",
    "Timing",
    "Wind",
    "Round",
    "Place",
    "Source",
    "Source ResultID",
    "Source URL",
];

fn write_partitioned_book(path: &Path, rows: &[PerformanceRow], per_sheet: usize) -> TestResult {
    let widths = [22_u16; 20];
    let header: Vec<Cell> = PARTITION_HEADER
        .iter()
        .map(|label| Cell::text(*label))
        .collect();
    let last_column = PARTITION_HEADER.len().saturating_sub(1);
    let mut book = Workbook::new();
    for (index, chunk) in rows.chunks(per_sheet).enumerate() {
        let name = format!("Part_{:03}", index.saturating_add(1));
        let mut sheet = SheetWriter::start(&mut book, path, &name, &widths)?;
        sheet.write_row(0, &header)?;
        for (offset, row) in chunk.iter().enumerate() {
            sheet.write_row(offset.saturating_add(1), &cells_of(row))?;
        }
        sheet.finish(chunk.len().saturating_add(1), last_column, true)?;
    }
    book.save(path)?;
    Ok(())
}

fn verify_partition_sheet(range: &Range<Data>, expected: &[PerformanceRow]) -> TestResult {
    check!(eq; range.height(), expected.len().saturating_add(1));
    for (column, label) in PARTITION_HEADER.iter().enumerate() {
        check!(eq; cell_text(range, 0, column), label.to_string());
    }
    for (offset, row) in expected.iter().enumerate() {
        for (column, value) in row.values().into_iter().enumerate() {
            let actual = cell_text(range, offset.saturating_add(1), column);
            check!(
                value.matches(&actual),
                "sheet cell ({offset}, {column}) reads back"
            );
        }
    }
    Ok(())
}

#[test]
fn performance_partitions_do_not_retain_prior_sheets() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    for fixture in &partition_fixtures() {
        seed_partition_store(&store, fixture)?;
    }
    let rows = projected_partition_rows(&store)?;
    check!(eq; rows.len(), 7);

    let path = dir.path().join("partitions.xlsx");
    write_partitioned_book(&path, &rows, 3)?;

    let mut book: Xlsx<_> = open_workbook(&path)?;
    check!(eq;
        book.sheet_names(),
        ["Part_001", "Part_002", "Part_003"]
    );
    for (index, chunk) in rows.chunks(3).enumerate() {
        let name = format!("Part_{:03}", index.saturating_add(1));
        let range = book.worksheet_range(&name)?;
        verify_partition_sheet(&range, chunk)?;
    }
    Ok(())
}

#[test]
fn sequential_writers_flush_completed_rows_to_bounded_memory() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("mode.xlsx");
    let widths = [18_u16, 18];
    let mut book = Workbook::new();
    let mut sheet = SheetWriter::start(&mut book, &path, "Mode", &widths)?;
    sheet.write_row(5, &[Cell::text("fifth"), Cell::text("row")])?;
    sheet.write_row(3, &[Cell::text("third"), Cell::text("row")])?;
    sheet.finish(6, 1, false)?;
    book.save(&path)?;

    let mut book: Xlsx<_> = open_workbook(&path)?;
    let range = book.worksheet_range("Mode")?;
    check!(eq; cell_text(&range, 5, 0), "fifth");
    check!(eq; cell_text(&range, 3, 0), String::new());
    Ok(())
}
