use std::path::Path;

use anyhow::{Context, Result};
use census_domain::model::GradYear;
use census_report::export::ExportDataset;
use census_report::report::Scope;
use census_report::workbook::publication::verify_for_seal;

use crate::census::WorkbookCheck;

pub fn inspect_workbook(
    path: &Path,
    dataset: &ExportDataset,
    cohort: GradYear,
    scope: Scope,
) -> Result<WorkbookCheck> {
    let verified = verify_for_seal(path, dataset, scope, cohort)
        .context("verifying the complete current census publication")?;
    Ok(WorkbookCheck {
        sheets: verified.workbook.sheets,
        rows: verified.workbook.rows,
        digests: vec![verified.generation_digest],
        mapped_athletes: verified.workbook.mapped_athletes,
        counts_reconciled: true,
        coverage_reconciled: true,
        metrics_reconciled: true,
        export_verified: true,
        discrepancies: Vec::new(),
    })
}
