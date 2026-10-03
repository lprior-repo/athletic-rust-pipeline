use super::*;
use crate::export::ExportDataset;
use crate::report::{Derivation, Scope};
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, CentiMetres, CentiSeconds, CompetitionLevel, EventKind, Evidence, Gender,
    GradYear, Mark, SchoolYear, SourceRef, Sport, TimingMethod,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const DAY: &str = "2026-09-21";
const SOURCE: &str = "wiaa_results";
const RESULT_URL: &str = "https://example.test/results/1";

struct Fixture {
    school: &'static str,
    athlete: &'static str,
    source_id: &'static str,
    date: &'static str,
    kind: EventKind,
    mark: Mark,
}

fn fixtures() -> Vec<Fixture> {
    vec![
        Fixture {
            school: "Abbotsford",
            athlete: "Ada",
            source_id: "runner-a",
            date: "2026-05-01",
            kind: EventKind::Track400m,
            mark: Mark::TimeSeconds(CentiSeconds::new(4855)),
        },
        Fixture {
            school: "Abbotsford",
            athlete: "Ada",
            source_id: "runner-a",
            date: "2026-05-08",
            kind: EventKind::Track400m,
            mark: Mark::TimeSeconds(CentiSeconds::new(4810)),
        },
        Fixture {
            school: "Abbotsford",
            athlete: "Bo",
            source_id: "runner-b",
            date: "2026-05-08",
            kind: EventKind::LongJump,
            mark: Mark::DistanceMetres(CentiMetres::new(642)),
        },
        Fixture {
            school: "Colby",
            athlete: "Cy",
            source_id: "runner-c",
            date: "2026-04-30",
            kind: EventKind::Track800m,
            mark: Mark::Raw("DNS".to_string()),
        },
        Fixture {
            school: "Colby",
            athlete: "Dee",
            source_id: "runner-d",
            date: "2026-05-08",
            kind: EventKind::Track1600m,
            mark: Mark::TimeSeconds(CentiSeconds::new(28123)),
        },
    ]
}

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

fn observation() -> Evidence {
    Evidence::parsed(SourceRef::new(SOURCE, Some(RESULT_URL.to_string())), DAY)
}

fn fixture_source(id: &str) -> census_domain::model::SourceIdentity {
    census_domain::model::SourceIdentity::new(
        census_domain::model::SourceNamespace::Other("fixture".to_owned()),
        id,
    )
}

fn seed(store: &Store, fixture: &Fixture) -> TestResult<String> {
    let (mut school, school_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        fixture.school,
        census_domain::model::normalize_name(fixture.school),
    );
    school.evidence.push(observation());
    store.append(Table::Schools, &school)?;

    let mut athlete = CanonicalAthlete::new(
        &school_id,
        fixture.athlete,
        GradYear::CO2027,
        Gender::Boys,
        fixture_source(fixture.source_id),
    );
    athlete.evidence.push(observation());
    store.append(Table::Athletes, &athlete)?;

    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "Invitational",
        fixture.date,
        CompetitionLevel::Invitational,
    );
    let meet_id = meet.id.clone();
    meet.evidence.push(observation());
    store.append(Table::Meets, &meet)?;

    let mut event = CanonicalEvent::new(
        &meet_id,
        fixture.kind.clone(),
        Gender::Boys,
        None,
        Some("Finals"),
    );
    let event_id = event.id.clone();
    event.evidence.push(observation());
    store.append(Table::Events, &event)?;

    let season = SchoolYear::new(2026).ok_or("invalid fixture season")?;
    let team = CanonicalTeam {
        id: CanonicalTeam::mint(&school_id, Sport::OutdoorTrack, Gender::Boys, season),
        school: school_id.clone(),
        sport: Sport::OutdoorTrack,
        gender: Gender::Boys,
        school_year: season,
        level: None,
        source_identities: Vec::new(),
        evidence: vec![observation()],
        retained_conflicts: Vec::new(),
    };
    store.append(Table::Teams, &team)?;

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
        team: team.id.clone(),
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
        source_athlete: athlete.source.clone(),
        retained_conflicts: Vec::new(),
    };
    store.append(Table::Performances, &performance)?;
    Ok(performance.id.as_str().to_string())
}

fn seeded_store(dir: &tempfile::TempDir, fixtures: &[Fixture]) -> TestResult<Store> {
    let store = Store::open(dir.path())?;
    for fixture in fixtures {
        seed(&store, fixture)?;
    }
    Ok(store)
}

fn write_sheets(rows: &[PerformanceRow], per_sheet: usize, path: &Path) -> TestResult {
    let mut book = Workbook::new();
    write_partitions(&mut book, path, rows.iter().cloned().map(Ok), per_sheet)?;
    book.save(path)?;
    Ok(())
}

fn performance_rows(store: &Store, scope: Scope) -> ReportResult<Vec<PerformanceRow>> {
    let dataset = ExportDataset::load(store)?;
    let derivation = Derivation::of(&dataset, scope, None);
    let lookups = super::join::PerformanceProjection::of(&derivation);
    let mut rows: Vec<PerformanceRow> = derivation
        .performances()
        .iter()
        .map(|performance| lookups.row(performance))
        .collect();
    rows.sort_by(super::rows::sheet_order);
    Ok(rows)
}

fn text(range: &Range<Data>, row: u32, column: u32) -> String {
    range
        .get_value((row, column))
        .map(|value| value.to_string())
        .map_or(Default::default(), core::convert::identity)
}

#[test]
fn sheet_names_zero_pad_to_three_digits_and_grow_past_them() {
    assert_eq!(sheet_name(0), "Performances_001");
    assert_eq!(sheet_name(41), "Performances_042");
    assert_eq!(sheet_name(998), "Performances_999");
    assert_eq!(sheet_name(999), "Performances_1000");
}

#[test]
fn a_row_prints_the_canonical_values_behind_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = seeded_store(&dir, &fixtures())?;
    let rows = performance_rows(&store, Scope::Core)?;
    let row = rows
        .iter()
        .find(|row| row.date == "2026-05-01")
        .ok_or("missing first May meet")?;

    check!(row.id.starts_with("perf_"));
    check!(row.athlete_id.starts_with("ath_"));
    check!(eq; row.athlete, "Ada");
    check!(eq; row.school, "Abbotsford");
    check!(eq; row.grad_year, Some(2027));
    check!(eq; row.meet, "Invitational");
    check!(eq; row.state.as_deref(), Some("WI"));
    check!(eq; row.sport, "Track");
    check!(eq; row.event, "Track400m");
    check!(eq; row.mark, "48.55");
    check!(eq; row.normalized, Some(48.55));
    check!(eq; row.timing.as_deref(), Some("Fat"));
    check!(eq; row.wind_mps, Some(1.4));
    check!(eq; row.round.as_deref(), Some("Finals"));
    check!(eq; row.place, Some(2));
    check!(eq; row.source, SOURCE);
    check!(eq; row.source_result, "test:Ada:2026-05-01");
    check!(eq; row.source_url, RESULT_URL);

    let raw = rows
        .iter()
        .find(|row| row.mark == "DNS")
        .ok_or("missing unparsed mark")?;
    check!(eq; raw.normalized, None);
    Ok(())
}

#[test]
fn two_partitions_repeat_the_header_and_split_the_sorted_rows() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = seeded_store(&dir, &fixtures())?;
    let rows = performance_rows(&store, Scope::Core)?;
    check!(eq; rows.len(), 5);

    let path = dir.path().join("book.xlsx");
    write_sheets(&rows, 2, &path)?;

    let mut book: Xlsx<_> = open_workbook(&path)?;
    let names: Vec<String> = ["Performances_001", "Performances_002", "Performances_003"]
        .iter()
        .map(|name| (*name).to_string())
        .collect();
    check!(eq; book.sheet_names(), names);

    let header: Vec<String> = COLUMNS
        .iter()
        .map(|(label, _)| (*label).to_string())
        .collect();
    let mut seen: Vec<String> = Vec::new();
    let columns = u32::try_from(COLUMNS.len())?;
    for (sheet, height) in names.iter().zip([3_usize, 3, 2]) {
        let range = book.worksheet_range(sheet)?;
        let printed: Vec<String> = (0..columns).map(|column| text(&range, 0, column)).collect();
        check!(eq; printed, header, "the header repeats on {sheet}");
        check!(eq; range.height(), height, "the split of {sheet}");
        let written = u32::try_from(range.height())?;
        for row in 1..written {
            seen.push(format!(
                "{}|{}|{}|{}",
                text(&range, row, 3),
                text(&range, row, 7),
                text(&range, row, 2),
                text(&range, row, 11)
            ));
        }
    }
    check!(eq; seen, expected_order());
    Ok(())
}

#[test]
fn the_same_rows_in_a_differently_ordered_store_partition_identically() -> TestResult {
    let mut reversed = fixtures();
    reversed.reverse();

    let first_dir = tempfile::tempdir()?;
    let second_dir = tempfile::tempdir()?;
    let first = seeded_store(&first_dir, &fixtures())?;
    let second = seeded_store(&second_dir, &reversed)?;

    let first_rows = performance_rows(&first, Scope::Core)?;
    let second_rows = performance_rows(&second, Scope::Core)?;
    check!(eq; first_rows, second_rows);
    check!(eq; first_rows, performance_rows(&first, Scope::Core)?);

    let first_path = first_dir.path().join("first.xlsx");
    let second_path = second_dir.path().join("second.xlsx");
    write_sheets(&first_rows, 2, &first_path)?;
    write_sheets(&second_rows, 2, &second_path)?;
    let mut first_book: Xlsx<_> = open_workbook(&first_path)?;
    let mut second_book: Xlsx<_> = open_workbook(&second_path)?;
    check!(eq; first_book.sheet_names(), second_book.sheet_names());
    check!(eq; first_book.worksheet_range("Performances_002")?.height(), 3);
    check!(eq; second_book.worksheet_range("Performances_002")?.height(), 3);
    Ok(())
}

#[test]
fn the_spilled_rows_match_the_collected_rows_across_range_seams() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = seeded_store(&dir, &fixtures())?;
    let collected = performance_rows(&store, Scope::Core)?;
    let dataset = ExportDataset::load(&store)?;
    let derivation = Derivation::of(&dataset, Scope::Core, None);

    let streamed: Vec<PerformanceRow> =
        PerformanceRows::with_ranges(&derivation, 2, 64)?.collect::<ReportResult<Vec<_>>>()?;
    check!(eq; streamed, collected, "the spill preserves the sheet order");

    let single: Vec<PerformanceRow> =
        PerformanceRows::with_ranges(&derivation, 1_000, 64)?.collect::<ReportResult<Vec<_>>>()?;
    check!(eq; single, collected, "one range reproduces the same order");

    let streamed_path = dir.path().join("streamed.xlsx");
    let mut book = Workbook::new();
    write_partitions(
        &mut book,
        &streamed_path,
        PerformanceRows::with_ranges(&derivation, 2, 64)?,
        2,
    )?;
    book.save(&streamed_path)?;
    let mut streamed_book: Xlsx<_> = open_workbook(&streamed_path)?;
    check!(eq;
        streamed_book.sheet_names(),
        ["Performances_001", "Performances_002", "Performances_003"]
    );
    let range = streamed_book.worksheet_range("Performances_003")?;
    check!(eq; range.height(), 2, "the last sheet holds the last row");
    check!(eq;
        text(&range, 1, 3),
        "Colby",
        "the streamed sheets carry the same rows in the same order"
    );
    Ok(())
}

#[test]
fn a_last_sheet_that_fills_exactly_is_not_followed_by_an_empty_one() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = seeded_store(&dir, &fixtures())?;
    let rows = performance_rows(&store, Scope::Core)?;
    let exactly_full = rows[..4].to_vec();

    let path = dir.path().join("full.xlsx");
    write_sheets(&exactly_full, 2, &path)?;

    let mut book: Xlsx<_> = open_workbook(&path)?;
    check!(eq; book.sheet_names(), ["Performances_001", "Performances_002"]);
    for sheet in ["Performances_001", "Performances_002"] {
        check!(eq; book.worksheet_range(sheet)?.height(), 3);
    }
    Ok(())
}

#[test]
fn a_budget_that_holds_no_row_is_rejected_naming_the_row() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = seeded_store(&dir, &fixtures())?;
    let rows = performance_rows(&store, Scope::Core)?;
    let first = rows.first().ok_or("missing first fixture row")?.id.clone();

    let mut book = Workbook::new();
    let path = dir.path().join("book.xlsx");
    let error = match write_partitions(&mut book, &path, rows.iter().cloned().map(Ok), 0) {
        Err(error) => error,
        Ok(_) => return Err("zero sheet budget accepted".into()),
    };
    check!(
        error.to_string().contains(&first),
        "the rejection names the row it could not write: {error}"
    );
    Ok(())
}

#[test]
fn a_spill_budget_that_holds_no_row_or_no_range_is_rejected() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = seeded_store(&dir, &fixtures())?;
    let dataset = ExportDataset::load(&store)?;
    let derivation = Derivation::of(&dataset, Scope::Core, None);

    for (range_rows, max_ranges) in [(0, 64), (2, 0)] {
        let error = match PerformanceRows::with_ranges(&derivation, range_rows, max_ranges) {
            Ok(_) => {
                return Err(format!(
                    "{range_rows} rows per range and {max_ranges} ranges were accepted"
                )
                .into())
            }
            Err(error) => error,
        };
        check!(
            error.to_string().contains("performance spill"),
            "the rejection names the spill budget: {error}"
        );
    }
    Ok(())
}

#[test]
fn a_performance_the_store_cannot_join_is_still_written() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (mut school, school_id) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Colby", "colby");
    school.evidence.push(observation());
    store.append(Table::Schools, &school)?;

    let source = fixture_source("orphan");
    let athlete = CanonicalAthlete::mint(
        &school_id,
        "Orphan",
        GradYear::CO2027,
        Gender::Boys,
        &source,
    );
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
            SchoolYear::new(2026).ok_or("invalid fixture season")?,
        ),
        event: CanonicalEvent::new(&meet, EventKind::Track800m, Gender::Boys, None, None).id,
        meet: meet.clone(),
        date: "2026-04-30".to_string(),
        mark: Mark::TimeSeconds(CentiSeconds::new(12050)),
        wind_mps: None,
        place: None,
        heat: None,
        round: None,
        timing: None,
        observed_grade: None,
        evidence: vec![observation()],
        source_key: "test:orphan".to_string(),
        source_athlete: Some(source),
        retained_conflicts: Vec::new(),
    };
    store.append(Table::Performances, &performance)?;

    let rows = performance_rows(&store, Scope::Core)?;
    check!(eq; rows.len(), 1);
    let row = rows.first().ok_or("missing orphan row")?;
    check!(eq; row.athlete_id, athlete.as_str());
    check!(eq; row.athlete, "");
    check!(eq; row.grad_year, None);
    check!(eq; row.meet_id, meet.as_str());
    check!(eq; row.meet, "");
    check!(eq; row.state, None);
    check!(eq; row.event, "");
    check!(eq; row.school, "");
    check!(eq; row.normalized, Some(120.5));

    let path = dir.path().join("book.xlsx");
    write_sheets(&rows, 2, &path)?;
    let mut book: Xlsx<_> = open_workbook(&path)?;
    check!(eq; book.sheet_names(), vec!["Performances_001".to_string()]);
    let range = book.worksheet_range("Performances_001")?;
    check!(eq; range.height(), 2);
    check!(eq; text(&range, 1, 1), athlete.as_str());
    Ok(())
}

#[test]
fn the_frozen_entry_point_writes_the_partitioned_sheets() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = seeded_store(&dir, &fixtures())?;
    let path = dir.path().join("book.xlsx");

    let dataset = ExportDataset::load(&store)?;
    let derivation = Derivation::of(&dataset, Scope::Core, None);
    let mut book = Workbook::new();
    write_performance_sheets(&mut book, &path, &derivation)?;
    book.save(&path)?;

    let mut book: Xlsx<_> = open_workbook(&path)?;
    check!(eq; book.sheet_names(), vec!["Performances_001".to_string()]);
    let range = book.worksheet_range("Performances_001")?;
    check!(eq; range.height(), 6);
    let printed: Vec<String> = (0..u32::try_from(COLUMNS.len())?)
        .map(|column| text(&range, 0, column))
        .collect();
    let header: Vec<String> = COLUMNS
        .iter()
        .map(|(label, _)| (*label).to_string())
        .collect();
    check!(eq; printed, header);
    Ok(())
}

mod affiliation;
