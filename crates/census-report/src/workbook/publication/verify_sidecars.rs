use crate::export::ExportDataset;
use crate::report::{ReportError, ReportResult};
use crate::workbook::Options;
use std::path::Path;

mod audit;
mod bests;
mod cells;
mod census;
mod contacts;
mod diff;
mod input;
mod read;
mod recruiting;
#[cfg(test)]
mod tests;

const MAX_DISPLAY_CHARS: usize = 160;

pub(super) fn verify(
    directory: &Path,
    dataset: &ExportDataset,
    options: &Options,
) -> ReportResult<()> {
    let inputs = input::Inputs::new(dataset, options)?;
    census::verify(directory, &inputs)?;
    recruiting::verify(directory, &inputs)?;
    contacts::verify(directory, &inputs)?;
    audit::verify(directory, &inputs)?;
    bests::verify(directory, &inputs)
}

pub(super) fn cohort(grad_year: Option<i16>) -> String {
    grad_year.map_or_else(|| "all".to_string(), |year| format!("co{year}"))
}

pub(super) fn defect(detail: String) -> ReportError {
    ReportError::Invariant { detail }
}

pub(super) fn excerpt(value: &str) -> String {
    let taken: String = value.chars().take(MAX_DISPLAY_CHARS).collect();
    if taken.len() < value.len() {
        format!("{taken}...")
    } else {
        taken
    }
}
