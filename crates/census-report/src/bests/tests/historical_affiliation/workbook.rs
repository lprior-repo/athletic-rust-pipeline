use super::*;
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use census_domain::model::SchoolYear;
use std::path::{Path, PathBuf};

mod tamper;

pub(super) fn publish(directory: &Path, fixture: &Fixture) -> TestResult<PathBuf> {
    let store = fixture.store(&directory.join("store"))?;
    crate::workbook::build(
        &store,
        &crate::workbook::Options {
            grad_year: Some(2027),
            out: Some(directory.join("publication")),
            limit: None,
            scope: Scope::AllSources,
            school_year: SchoolYear::new(2026).ok_or("invalid publication season")?,
        },
    )
    .map_err(Into::into)
}

pub(super) fn cell<'a>(range: &'a Range<Data>, name: &str) -> TestResult<&'a Data> {
    let headers = range
        .rows()
        .next()
        .ok_or("missing historical-affiliation XLSX header")?;
    let column = headers
        .iter()
        .position(|value| value == name)
        .ok_or_else(|| format!("missing historical-affiliation XLSX column {name}"))?;
    range
        .get((1, column))
        .ok_or_else(|| format!("missing XLSX cell {name}").into())
}

fn xlsx_cells(range: &Range<Data>, fields: &[(&str, &str)]) -> TestResult {
    check!(eq; range.height(), 2);
    fields
        .iter()
        .try_for_each(|(header, expected)| -> TestResult {
            check!(eq; cell(range, header)?.to_string(), *expected, "{header}");
            Ok(())
        })
}

fn assert_prs(range: &Range<Data>, fixture: &Fixture) -> TestResult {
    xlsx_cells(
        range,
        &[
            ("Athlete ID", fixture.athlete.id.as_str()),
            ("Athlete", ATHLETE_NAME),
            ("School", fixture.case.school_text()),
            ("State", fixture.case.state().code()),
            ("Calculated PR", "48.55"),
            ("Unit", "s"),
            ("PR date", RESULT_DATE),
            ("Meet", fixture.meet.name.as_str()),
            ("Result URL", RESULT_URL),
            ("Performance ID", fixture.performance.id.as_str()),
            ("Meet ID", fixture.meet.id.as_str()),
            ("Source Key", SOURCE_KEY),
            ("Surface", "outdoor"),
            ("Timing Class", "fat"),
        ],
    )?;
    check!(eq; cell(range, "Mark Value")?, &Data::Float(48.55));
    check!(eq; cell(range, "Graduation Year")?, &Data::Float(2027.0));
    check!(eq; cell(range, "Source count")?, &Data::Float(1.0));
    Ok(())
}

fn assert_current_directory(path: &Path, generation: &Path, fixture: &Fixture) -> TestResult {
    let mut book: Xlsx<_> = open_workbook(path)?;
    let athletes = book.worksheet_range("Athletes")?;
    xlsx_cells(
        &athletes,
        &[
            ("Athlete ID", fixture.athlete.id.as_str()),
            ("Name", ATHLETE_NAME),
            ("School", CURRENT_SCHOOL),
            ("School ID", fixture.current.id.as_str()),
            ("State", "IL"),
            ("MileSplit URL", PROFILE_URL),
            ("Headline PR summary", "400m 48.55 [outdoor, na, fat]"),
            ("Coverage State", "pr"),
        ],
    )?;
    artifacts::csv_cells(
        &generation.join("recruiting.csv"),
        &[
            ("athlete_id", fixture.athlete.id.as_str()),
            ("name", ATHLETE_NAME),
            ("school", CURRENT_SCHOOL),
            ("state", "IL"),
            ("grad_year", "2027"),
            ("milesplit_url", PROFILE_URL),
        ],
    )
}

fn assert_case(case: AffiliationCase) -> TestResult {
    let fixture = Fixture::new(case)?;
    let directory = tempfile::tempdir()?;
    let path = publish(directory.path(), &fixture)?;
    let generation = path
        .parent()
        .ok_or("missing historical-affiliation generation")?;
    let frozen = ExportDataset::reopen_frozen(&generation.join("frozen-input.json"))?;
    assert_selection(&frozen, &fixture, Scope::AllSources)?;
    drop(frozen);
    let verified = crate::workbook::publication::verify_published(&path)?;
    check!(eq; verified.workbook.mapped_athletes, 1);
    let mut book: Xlsx<_> = open_workbook(&path)?;
    assert_prs(&book.worksheet_range("PRs")?, &fixture)?;
    artifacts::assert_bests(generation, &fixture)?;
    assert_current_directory(&path, generation, &fixture)
}

#[test]
fn published_bests_keep_unknown_affiliation_when_the_performance_team_is_missing() -> TestResult {
    assert_case(AffiliationCase::MissingTeam)
}

#[test]
fn published_bests_keep_unknown_affiliation_when_the_historical_school_is_missing() -> TestResult {
    assert_case(AffiliationCase::MissingSchool)
}

#[test]
fn published_bests_keep_the_historical_name_when_its_state_is_unknown() -> TestResult {
    assert_case(AffiliationCase::UnknownHistoricalState)
}

#[test]
fn published_bests_keep_wisconsin_affiliation_after_a_transfer_to_illinois() -> TestResult {
    assert_case(AffiliationCase::CrossStateTransfer)
}
