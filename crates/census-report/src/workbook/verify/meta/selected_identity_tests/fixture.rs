use super::{expected_sheets, Expect, Expectations, TestResult};
use crate::export::ExportDataset;
use crate::report::{Derivation, EventAdmission, Scope};
use crate::workbook::meta::{write_meta_sheets, RunFacts};
use crate::workbook::verify::{read::Book, report::Findings};
use crate::workbook::{Censuses, Options};
use census_domain::model::SchoolYear;
use rust_xlsxwriter::Workbook;
use std::path::Path;

pub(super) fn options() -> TestResult<Options> {
    Ok(Options {
        grad_year: None,
        out: None,
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: Vec::new(),
        school_year: SchoolYear::new(2026).ok_or("fixture school year")?,
    })
}

pub(super) fn expected(dataset: &ExportDataset, name: &str) -> TestResult<Vec<Vec<String>>> {
    let expectations = Expectations::of(dataset, &options()?)?;
    let (_, rows) = expected_sheets(&expectations)?
        .into_iter()
        .find(|(sheet, _)| *sheet == name)
        .ok_or("metadata sheet")?;
    Ok(rows
        .into_iter()
        .map(|row| {
            row.into_iter()
                .map(|cell| match cell {
                    Expect::Text(value) => value,
                    Expect::Number(value) => value.to_string(),
                    Expect::Empty => String::new(),
                })
                .collect()
        })
        .collect())
}

pub(super) fn field<'a>(
    rows: &'a [Vec<String>],
    label: &str,
    column: usize,
) -> TestResult<&'a str> {
    rows.iter()
        .rev()
        .find(|row| row.first().is_some_and(|cell| cell == label))
        .and_then(|row| row.get(column))
        .map(String::as_str)
        .ok_or_else(|| format!("missing {label} column {column}").into())
}

pub(super) fn write(dataset: &ExportDataset, path: &Path) -> TestResult {
    let options = options()?;
    let population = Derivation::for_publication(dataset, Scope::AllSources, None, &[])?;
    let censuses = Censuses::of(dataset, Path::new("synthetic-output"), &[])?;
    let admission = EventAdmission::of(dataset)?;
    let mut book = Workbook::new();
    write_meta_sheets(
        &mut book,
        path,
        RunFacts {
            population: &population,
            recruiting: &population,
            core: &censuses.core,
            all_sources: &censuses.all_sources,
            bests: &[],
            school_year: options.school_year,
            admission: &admission,
        },
    )?;
    book.save(path)?;
    Ok(())
}

pub(super) fn validate(dataset: &ExportDataset, path: &Path) -> TestResult {
    let expectations = Expectations::of(dataset, &options()?)?;
    let mut book = Book::open(path)?;
    let mut findings = Findings::default();
    super::super::verify(&mut book, &expectations, &mut findings)?;
    findings.finish(path)?;
    Ok(())
}

pub(super) fn reject(
    dataset: &ExportDataset,
    directory: &Path,
    name: &str,
    rows: &[Vec<Expect>],
    location: &str,
) -> TestResult {
    let path = directory.join("tampered-selected-meta.xlsx");
    let mut book = Workbook::new();
    let sheet = book.add_worksheet();
    sheet.set_name(name)?;
    for (row, cells) in rows.iter().enumerate() {
        for (column, cell) in cells.iter().enumerate() {
            let (row, column) = (u32::try_from(row)?, u16::try_from(column)?);
            match cell {
                Expect::Text(value) => {
                    sheet.write_string(row, column, value)?;
                }
                Expect::Number(value) => {
                    sheet.write_number(row, column, *value)?;
                }
                Expect::Empty => {}
            }
        }
    }
    book.save(&path)?;
    let expectations = Expectations::of(dataset, &options()?)?;
    let (_, expected) = expected_sheets(&expectations)?
        .into_iter()
        .find(|(sheet, _)| *sheet == name)
        .ok_or("metadata sheet")?;
    let mut findings = Findings::default();
    super::super::readback(&mut Book::open(&path)?, name, &expected, &mut findings)?;
    let error = findings
        .finish(&path)
        .err()
        .ok_or("mutated selected metadata accepted")?;
    check!(error.to_string().contains(location));
    Ok(())
}
