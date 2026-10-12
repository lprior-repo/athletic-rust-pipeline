use std::path::Path;

use anyhow::{Context, Result};
use census_domain::model::GradYear;
use census_report::export::{store_lineage, ExportDataset};
use census_report::report::Scope;
use census_report::workbook::publication::open_publication;
use census_store::Store;

use crate::census::WorkbookCheck;

pub struct Inspection {
    pub dataset: ExportDataset,
    pub check: WorkbookCheck,
}

pub fn inspect_workbook(
    path: &Path,
    store: &Store,
    cohort: GradYear,
    scope: Scope,
) -> Result<Inspection> {
    let lineage = store_lineage(store).context("measuring the store's current generation")?;
    let opened = open_publication(path, &lineage, scope, cohort)
        .context("verifying the complete current census publication")?;
    let dataset = opened.dataset;
    let sheets = opened.workbook.sheets;
    let rows = opened.workbook.rows;
    let mapped_athletes = opened.workbook.mapped_athletes;
    let generation_digest = opened.generation_digest;
    Ok(Inspection {
        dataset,
        check: WorkbookCheck {
            sheets,
            rows,
            digests: vec![generation_digest],
            mapped_athletes,
            counts_reconciled: true,
            coverage_reconciled: true,
            metrics_reconciled: true,
            export_verified: true,
            discrepancies: Vec::new(),
        },
    })
}
