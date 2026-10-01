use crate::export::ExportDataset;
use crate::report::ReportResult;
use crate::workbook::Options;
use std::path::Path;

mod athletes;
mod canonical;
mod coaches;
mod expectations;
mod labels;
mod meta;
mod performances;
mod prs;
mod reach;
mod read;
mod report;

#[derive(Debug)]
pub struct VerifiedWorkbook {
    pub sheets: u64,
    pub rows: u64,
    pub mapped_athletes: u64,
}

pub fn verify_frozen(
    path: &Path,
    dataset: &ExportDataset,
    options: &Options,
) -> ReportResult<VerifiedWorkbook> {
    let expectations = expectations::Expectations::of(dataset, options)?;
    let mut book = read::Book::open(path)?;
    let mut findings = report::Findings::default();
    read::verify_inventory(&book, &expectations, &mut findings);
    performances::verify(&mut book, &expectations, &mut findings)?;
    athletes::verify(&mut book, &expectations, &mut findings)?;
    prs::verify(&mut book, &expectations, &mut findings)?;
    coaches::verify(&mut book, &expectations, &mut findings)?;
    meta::verify(&mut book, &expectations, &mut findings)?;
    let rows = findings.finish()?;
    Ok(VerifiedWorkbook {
        sheets: count(book.names().len())?,
        rows: count(rows)?,
        mapped_athletes: count(expectations.derivation.athletes().len())?,
    })
}

fn count(value: usize) -> ReportResult<u64> {
    u64::try_from(value).map_err(|_| crate::report::ReportError::Invariant {
        detail: "verified workbook count exhausted".to_string(),
    })
}
