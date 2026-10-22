pub(super) mod current_school;
use crate::export::ExportDataset;
use crate::report::ReportResult;
use crate::workbook::Options;
use std::path::Path;

mod athletes;
pub(in crate::workbook) mod canonical;
mod coaches;
mod context;
mod expectations;
mod labels;
pub(in crate::workbook) mod link;
pub(in crate::workbook) mod meta;
mod performances;
pub(in crate::workbook) mod postal;
mod prs;
pub(in crate::workbook) mod public_identity;
mod reach;
mod read;
mod relays;
mod report;
mod reported;
mod worksheet;
pub(in crate::workbook) mod xlsx;

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
    super::publication::selected::ensure_verification_options(dataset, options)?;
    let expectations = expectations::Expectations::of(dataset, options)?;
    let mut book = read::Book::open(path)?;
    let mut findings = report::Findings::default();
    read::verify_inventory(&book, &expectations, &mut findings);
    if dataset.selected_profile_id().is_some() && !dataset.is_historical_selected_replay() {
        worksheet::verify(&mut book, &expectations, &mut findings)?;
        return Ok(VerifiedWorkbook {
            sheets: count(book.names().len())?,
            rows: count(findings.finish(path)?)?,
            mapped_athletes: count(expectations.derivation.athletes().len())?,
        });
    }
    performances::verify(&mut book, &expectations, &mut findings)?;
    relays::verify(&mut book, &expectations, &mut findings)?;
    if dataset.selected_profile_id().is_some()
        && dataset
            .selected_authority()?
            .profile()
            .input()
            .include_provisional
            == census_domain::model::ProvisionalInclusionV1::Include
    {
        reported::verify(&mut book, &expectations, &mut findings)?;
    } else {
        athletes::verify(&mut book, &expectations, &mut findings)?;
        prs::verify(&mut book, &expectations, &mut findings)?;
    }
    coaches::verify(&mut book, &expectations, &mut findings)?;
    meta::verify(&mut book, &expectations, &mut findings)?;
    let rows = findings.finish(path)?;
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
