use super::super::source::TestResult;
use super::{artifacts, Expected};
use crate::export::ExportDataset;
use calamine::Reader;
use census_domain::model::CanonicalPerformance;
use census_domain::UsJurisdiction;
use std::path::Path;

const CURRENT_SCHOOL: &str = "Controlled Colorado High School";
const HISTORICAL_SCHOOL: &str = "Detroit Catholic Central";
const CUTOFF: &str = "2026-10-16";

pub(super) fn matches(
    dataset: &ExportDataset,
    workbook: &Path,
    wanted: &CanonicalPerformance,
    expected: &Expected<'_>,
) -> TestResult<()> {
    let current = dataset
        .schools
        .get(expected.current_school)
        .ok_or("current school")?;
    let historical = dataset
        .schools
        .get(expected.historical_school)
        .ok_or("historical school")?;
    check!(eq; current.name, CURRENT_SCHOOL);
    check!(eq; historical.name, HISTORICAL_SCHOOL);
    check!(eq; current.state, Some(UsJurisdiction::Colorado));
    check!(eq; historical.state, Some(UsJurisdiction::Michigan));
    check!(eq; dataset.selected_authority()?.profile().input().as_of.as_str(), CUTOFF);
    check!(std::fs::metadata(workbook)?.len() <= 16 * 1024 * 1024);
    let mut book: calamine::Xlsx<std::io::BufReader<std::fs::File>> =
        calamine::open_workbook(workbook)?;
    check!(eq; book.sheet_names(), vec!["Athletes".to_owned()]);
    pr_sheet(&mut book, wanted, expected)?;
    athletes_sheet(&mut book, expected)?;
    recruiting(workbook, expected)
}

fn pr_sheet(
    book: &mut calamine::Xlsx<std::io::BufReader<std::fs::File>>,
    wanted: &CanonicalPerformance,
    expected: &Expected<'_>,
) -> TestResult<()> {
    let rows = artifacts::sheet(book, "Athletes")?;
    check!(eq; rows.len(), 1);
    let row = rows.first().ok_or("athlete bests row")?;
    artifacts::fields(
        row,
        &[
            ("Athlete ID", expected.public.as_str()),
            ("Current school", CURRENT_SCHOOL),
            ("Current state", "CO"),
            ("Current school as-of", CUTOFF),
        ],
    )?;
    let bests = row.get("Observed bests").ok_or("observed bests")?;
    for (label, value) in [
        ("Calculated PR", expected.seconds),
        ("PR date", expected.day),
        ("Performance ID", wanted.id.as_str()),
        ("Source Key", wanted.source_key.as_str()),
        ("School", HISTORICAL_SCHOOL),
        ("State", "MI"),
    ] {
        check!(bests.contains(&format!("{label}: {value}")), "{label}");
    }
    Ok(())
}

fn athletes_sheet(
    book: &mut calamine::Xlsx<std::io::BufReader<std::fs::File>>,
    expected: &Expected<'_>,
) -> TestResult<()> {
    let rows = artifacts::sheet(book, "Athletes")?;
    check!(eq; rows.len(), 1);
    artifacts::fields(
        rows.first().ok_or("athletes row")?,
        &[
            ("Athlete ID", expected.public.as_str()),
            ("School ID", expected.current_school.as_str()),
            ("School", CURRENT_SCHOOL),
            ("School State", "CO"),
        ],
    )
}

fn recruiting(workbook: &Path, expected: &Expected<'_>) -> TestResult<()> {
    let directory = workbook.parent().ok_or("publication directory")?;
    let rows = artifacts::csv_rows(&directory.join("recruiting.csv"))?;
    check!(eq; rows.len(), 1);
    artifacts::fields(
        rows.first().ok_or("recruiting row")?,
        &[
            ("athlete_id", expected.public.as_str()),
            ("school", CURRENT_SCHOOL),
            ("state", "CO"),
        ],
    )
}
