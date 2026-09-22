//! The §52 partition: the column order, the budget and its margin, the zero-padded sheet names,
//! the repeated header, the order the split follows, and the rejection that keeps a row from being
//! dropped.

use super::*;
use crate::store::Table;
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, CompetitionLevel, EventKind, Evidence, Gender, GradYear, Mark, SchoolYear,
    SourceRef, Sport, TimingMethod,
};
use census_domain::UsJurisdiction;

const DAY: &str = "2026-09-21";
const SOURCE: &str = "wiaa_results";
const RESULT_URL: &str = "https://example.test/results/1";

/// One synthetic performance and the rows it needs, small enough to read whole.
struct Fixture {
    school: &'static str,
    athlete: &'static str,
    date: &'static str,
    kind: EventKind,
    mark: Mark,
}

/// The five performances the split tests run on: two schools, three dates, a time, a distance and a
/// mark the census has not parsed.
fn fixtures() -> Vec<Fixture> {
    vec![
        Fixture {
            school: "Abbotsford",
            athlete: "Ada",
            date: "2026-05-01",
            kind: EventKind::Track400m,
            mark: Mark::TimeSeconds(48.55),
        },
        Fixture {
            school: "Abbotsford",
            athlete: "Ada",
            date: "2026-05-08",
            kind: EventKind::Track400m,
            mark: Mark::TimeSeconds(48.10),
        },
        Fixture {
            school: "Abbotsford",
            athlete: "Bo",
            date: "2026-05-08",
            kind: EventKind::LongJump,
            mark: Mark::DistanceMetres(6.42),
        },
        Fixture {
            school: "Colby",
            athlete: "Cy",
            date: "2026-04-30",
            kind: EventKind::Track800m,
            mark: Mark::Raw("DNS".to_string()),
        },
        Fixture {
            school: "Colby",
            athlete: "Dee",
            date: "2026-05-08",
            kind: EventKind::Track1600m,
            mark: Mark::TimeSeconds(281.23),
        },
    ]
}

/// The order the sheets publish: school, then date, then athlete (`COLUMNS` columns 2, 6, 1, 10).
fn expected_order() -> Vec<String> {
    [
        ("Abbotsford", "2026-05-01", "Ada", "48.55"),
        ("Abbotsford", "2026-05-08", "Ada", "48.10"),
        ("Abbotsford", "2026-05-08", "Bo", "6.42 m"),
        ("Colby", "2026-04-30", "Cy", "DNS"),
        ("Colby", "2026-05-08", "Dee", "4:41.23"),
    ]
    .iter()
    .map(|(school, date, athlete, mark)| format!("{school}|{date}|{athlete}|{mark}"))
    .collect()
}

/// One core-source observation, carrying the URL the `Source URL` column prints.
fn observation() -> Evidence {
    Evidence::parsed(SourceRef::new(SOURCE, Some(RESULT_URL.to_string())), DAY)
}

/// Append a fixture's school, athlete, meet, event and performance, returning the performance id.
fn seed(store: &Store, fixture: &Fixture) -> String {
    let (mut school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        fixture.school,
        &census_domain::model::normalize_name(fixture.school),
    );
    school.evidence.push(observation());
    store.append(Table::Schools, &school).unwrap();

    let mut athlete =
        CanonicalAthlete::new(&school_id, fixture.athlete, GradYear::CO2027, Gender::Boys);
    athlete.evidence.push(observation());
    store.append(Table::Athletes, &athlete).unwrap();

    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "Invitational",
        fixture.date,
        CompetitionLevel::Invitational,
    );
    let meet_id = meet.id.clone();
    meet.evidence.push(observation());
    store.append(Table::Meets, &meet).unwrap();

    let mut event = CanonicalEvent::new(
        &meet_id,
        fixture.kind.clone(),
        Gender::Boys,
        None,
        Some("Finals"),
    );
    let event_id = event.id.clone();
    event.evidence.push(observation());
    store.append(Table::Events, &event).unwrap();

    let source_key = format!("test:{}:{}", fixture.athlete, fixture.date);
    let performance = CanonicalPerformance {
        id: CanonicalPerformance::mint(
            &athlete.id,
            &meet_id,
            &fixture.kind,
            fixture.date,
            &source_key,
        ),
        athlete: athlete.id.clone(),
        team: CanonicalTeam::mint(
            &school_id,
            Sport::OutdoorTrack,
            Gender::Boys,
            SchoolYear(2026),
        ),
        event: event_id,
        meet: meet_id,
        date: fixture.date.to_string(),
        mark: fixture.mark.clone(),
        wind_mps: Some(1.4),
        place: Some(2),
        heat: None,
        round: None,
        timing: Some(TimingMethod::Fat),
        observed_grade: None,
        evidence: vec![observation()],
        source_key,
    };
    store.append(Table::Performances, &performance).unwrap();
    performance.id.as_str().to_string()
}

/// A store holding [`fixtures`], in the order given.
fn seeded_store(dir: &tempfile::TempDir, fixtures: &[Fixture]) -> Store {
    let store = Store::open(dir.path()).unwrap();
    for fixture in fixtures {
        seed(&store, fixture);
    }
    store
}

/// Write `rows` to `path`, `per_sheet` data rows to a sheet.
fn write_sheets(rows: &[PerformanceRow], per_sheet: usize, path: &Path) {
    let mut book = Workbook::new();
    write_partitions(&mut book, path, rows, per_sheet).unwrap();
    book.save(path).unwrap();
}

/// The text one cell holds, as the sheet publishes it. `calamine` addresses cells by `(u32, u32)`.
fn text(range: &Range<Data>, row: u32, column: u32) -> String {
    range
        .get_value((row, column))
        .map(|value| value.to_string())
        .unwrap_or_default()
}

#[test]
fn the_columns_are_the_objectives_order_and_the_budget_keeps_the_margin() {
    let labels: Vec<&str> = COLUMNS.iter().map(|(label, _)| *label).collect();
    assert_eq!(
        labels,
        [
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
        ]
    );
    // The stated margin: a sheet writes its header plus at most one budget of data rows, and spends
    // PARTITION_MARGIN rows less than Excel's own cap on that.
    assert_eq!(DATA_ROWS_PER_SHEET, 1_000_000);
    assert_eq!(
        EXCEL_ROWS_PER_SHEET - DATA_ROWS_PER_SHEET - HEADER_ROWS,
        PARTITION_MARGIN
    );
}

#[test]
fn sheet_names_zero_pad_to_three_digits_and_grow_past_them() {
    assert_eq!(sheet_name(0), "Performances_001");
    assert_eq!(sheet_name(41), "Performances_042");
    assert_eq!(sheet_name(998), "Performances_999");
    assert_eq!(sheet_name(999), "Performances_1000");
}

#[test]
fn a_row_prints_the_canonical_values_behind_it() {
    let dir = tempfile::tempdir().unwrap();
    let store = seeded_store(&dir, &fixtures());
    let rows = performance_rows(&store, Scope::Core).unwrap();
    let row = rows
        .iter()
        .find(|row| row.date == "2026-05-01")
        .expect("the first May meet");

    assert!(row.id.starts_with("perf_"));
    assert!(row.athlete_id.starts_with("ath_"));
    assert_eq!(row.athlete, "Ada");
    assert_eq!(row.school, "Abbotsford");
    assert_eq!(row.grad_year, Some(2027));
    assert_eq!(row.meet, "Invitational");
    assert_eq!(row.state.as_deref(), Some("WI"));
    assert_eq!(row.sport, "Track");
    assert_eq!(row.event, "Track400m");
    assert_eq!(row.mark, "48.55");
    assert_eq!(row.normalized, Some(48.55));
    assert_eq!(row.timing.as_deref(), Some("Fat"));
    assert_eq!(row.wind_mps, Some(1.4));
    // The performance published no round of its own, so the round of its event is what the sheet
    // prints rather than a blank.
    assert_eq!(row.round.as_deref(), Some("Finals"));
    assert_eq!(row.place, Some(2));
    assert_eq!(row.source, SOURCE);
    assert_eq!(row.source_result, "test:Ada:2026-05-01");
    assert_eq!(row.source_url, RESULT_URL);

    // A mark the census has not parsed is carried as published and normalizes to nothing, rather
    // than to a number nobody measured.
    let raw = rows
        .iter()
        .find(|row| row.mark == "DNS")
        .expect("the unparsed mark");
    assert_eq!(raw.normalized, None);
}

#[test]
fn two_partitions_repeat_the_header_and_split_the_sorted_rows() {
    let dir = tempfile::tempdir().unwrap();
    let store = seeded_store(&dir, &fixtures());
    let rows = performance_rows(&store, Scope::Core).unwrap();
    assert_eq!(rows.len(), 5);

    let path = dir.path().join("book.xlsx");
    write_sheets(&rows, 2, &path);

    let mut book: Xlsx<_> = open_workbook(&path).unwrap();
    let names: Vec<String> = ["Performances_001", "Performances_002", "Performances_003"]
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    assert_eq!(book.sheet_names(), names);

    let header: Vec<String> = COLUMNS
        .iter()
        .map(|(label, _)| (*label).to_string())
        .collect();
    let mut seen: Vec<String> = Vec::new();
    let columns = u32::try_from(COLUMNS.len()).unwrap();
    for (sheet, height) in names.iter().zip([3_usize, 3, 2]) {
        let range = book.worksheet_range(sheet).unwrap();
        let printed: Vec<String> = (0..columns).map(|column| text(&range, 0, column)).collect();
        assert_eq!(printed, header, "the header repeats on {sheet}");
        assert_eq!(range.height(), height, "the split of {sheet}");
        let written = u32::try_from(range.height()).unwrap();
        for row in 1..written {
            seen.push(format!(
                "{}|{}|{}|{}",
                text(&range, row, 2),
                text(&range, row, 6),
                text(&range, row, 1),
                text(&range, row, 10)
            ));
        }
    }
    assert_eq!(seen, expected_order());
}

#[test]
fn the_same_rows_in_a_differently_ordered_store_partition_identically() {
    let mut reversed = fixtures();
    reversed.reverse();

    let first_dir = tempfile::tempdir().unwrap();
    let second_dir = tempfile::tempdir().unwrap();
    let first = seeded_store(&first_dir, &fixtures());
    let second = seeded_store(&second_dir, &reversed);

    let first_rows = performance_rows(&first, Scope::Core).unwrap();
    let second_rows = performance_rows(&second, Scope::Core).unwrap();
    assert_eq!(first_rows, second_rows);
    // A re-run over the same store produces the same rows, and therefore the same partitioning.
    assert_eq!(first_rows, performance_rows(&first, Scope::Core).unwrap());

    // The same rows, so the same split and the same sheet names.
    let first_path = first_dir.path().join("first.xlsx");
    let second_path = second_dir.path().join("second.xlsx");
    write_sheets(&first_rows, 2, &first_path);
    write_sheets(&second_rows, 2, &second_path);
    let mut first_book: Xlsx<_> = open_workbook(&first_path).unwrap();
    let mut second_book: Xlsx<_> = open_workbook(&second_path).unwrap();
    assert_eq!(first_book.sheet_names(), second_book.sheet_names());
    assert_eq!(
        first_book
            .worksheet_range("Performances_002")
            .unwrap()
            .height(),
        3
    );
    assert_eq!(
        second_book
            .worksheet_range("Performances_002")
            .unwrap()
            .height(),
        3
    );
}

#[test]
fn a_budget_that_holds_no_row_is_rejected_naming_the_row() {
    let dir = tempfile::tempdir().unwrap();
    let store = seeded_store(&dir, &fixtures());
    let rows = performance_rows(&store, Scope::Core).unwrap();
    let first = rows.first().expect("a first row").id.clone();

    let mut book = Workbook::new();
    let path = dir.path().join("book.xlsx");
    let error = write_partitions(&mut book, &path, &rows, 0).unwrap_err();
    assert!(
        error.to_string().contains(&first),
        "the rejection names the row it could not write: {error}"
    );
}

#[test]
fn a_performance_the_store_cannot_join_is_still_written() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let (mut school, school_id) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Colby", "colby");
    school.evidence.push(observation());
    store.append(Table::Schools, &school).unwrap();

    // A performance whose athlete, meet and event rows the store never received: the ids are all it
    // has, and the sheet prints them rather than dropping the mark.
    let athlete = CanonicalAthlete::mint(&school_id, "Orphan", GradYear::CO2027, Gender::Boys);
    let meet = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-04-30",
        "Unreported Invitational",
        None,
    );
    let performance = CanonicalPerformance {
        id: CanonicalPerformance::mint(
            &athlete,
            &meet,
            &EventKind::Track800m,
            "2026-04-30",
            "test:orphan",
        ),
        athlete: athlete.clone(),
        team: CanonicalTeam::mint(
            &school_id,
            Sport::OutdoorTrack,
            Gender::Boys,
            SchoolYear(2026),
        ),
        event: CanonicalEvent::new(&meet, EventKind::Track800m, Gender::Boys, None, None).id,
        meet: meet.clone(),
        date: "2026-04-30".to_string(),
        mark: Mark::TimeSeconds(120.5),
        wind_mps: None,
        place: None,
        heat: None,
        round: None,
        timing: None,
        observed_grade: None,
        evidence: vec![observation()],
        source_key: "test:orphan".to_string(),
    };
    store.append(Table::Performances, &performance).unwrap();

    let rows = performance_rows(&store, Scope::Core).unwrap();
    assert_eq!(rows.len(), 1);
    let row = rows.first().expect("the orphan row");
    assert_eq!(row.athlete_id, athlete.as_str());
    assert_eq!(row.athlete, "");
    assert_eq!(row.grad_year, None);
    assert_eq!(row.meet_id, meet.as_str());
    assert_eq!(row.meet, "");
    assert_eq!(row.state, None);
    assert_eq!(row.event, "");
    assert_eq!(row.normalized, Some(120.5));

    let path = dir.path().join("book.xlsx");
    write_sheets(&rows, 2, &path);
    let mut book: Xlsx<_> = open_workbook(&path).unwrap();
    assert_eq!(book.sheet_names(), vec!["Performances_001".to_string()]);
    let range = book.worksheet_range("Performances_001").unwrap();
    assert_eq!(range.height(), 2);
    assert_eq!(text(&range, 1, 0), athlete.as_str());
}

#[test]
fn the_frozen_entry_point_writes_the_partitioned_sheets() {
    let dir = tempfile::tempdir().unwrap();
    let store = seeded_store(&dir, &fixtures());
    let path = dir.path().join("book.xlsx");

    let mut book = Workbook::new();
    write_performance_sheets(&mut book, &path, &store, Scope::Core).unwrap();
    book.save(&path).unwrap();

    // Five rows under the real 1,000,000-row budget are one sheet, and it carries the header the
    // objective lists plus one row per stored performance.
    let mut book: Xlsx<_> = open_workbook(&path).unwrap();
    assert_eq!(book.sheet_names(), vec!["Performances_001".to_string()]);
    let range = book.worksheet_range("Performances_001").unwrap();
    assert_eq!(range.height(), 6);
    let printed: Vec<String> = (0..u32::try_from(COLUMNS.len()).unwrap())
        .map(|column| text(&range, 0, column))
        .collect();
    let header: Vec<String> = COLUMNS
        .iter()
        .map(|(label, _)| (*label).to_string())
        .collect();
    assert_eq!(printed, header);
}
