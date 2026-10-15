use crate::export::ExportDataset;
use crate::report::{ReportError, ReportResult};
use crate::workbook::Options;
use std::path::Path;

mod audit;
mod bests;
mod catalog;
mod cells;
mod census;
mod contacts;
mod diff;
mod input;
mod normalized;
mod read;
mod recruiting;
#[cfg(test)]
mod tests;

const MAX_DISPLAY_CHARS: usize = 160;

pub(super) fn verify(
    directory: &Path,
    dataset: &ExportDataset,
    options: &Options,
    artifact_revision: u32,
) -> ReportResult<super::inventory::RowCounts> {
    super::versions::validate_policy(artifact_revision, dataset.lineage.policy_revision)?;
    let inputs = input::Inputs::new(dataset, options)?;
    catalog::verify(directory, &inputs)?;
    census::verify(directory, &inputs)?;
    recruiting::verify(directory, &inputs)?;
    contacts::verify(directory, &inputs)?;
    audit::verify(directory, &inputs)?;
    let outcome = crate::bests::build_admitted_outcome(
        &inputs.admission,
        &crate::bests::Options {
            scope: options.scope,
            grad_year: options.grad_year,
            limit: options.limit,
            jurisdictions: options.jurisdictions.clone(),
        },
    )?;
    bests::verify(directory, &inputs, artifact_revision, &outcome.selections)?;
    if matches!(artifact_revision, 3..=6) {
        return normalized::verify_outcome(directory, &inputs, artifact_revision, &outcome);
    }
    Ok(super::inventory::RowCounts::new())
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
