use std::path::Path;

use anyhow::{Context, Result};
use census_domain::model::GradYear;
use census_report::export::{store_lineage, ExportDataset};
use census_report::report::Scope;
use census_report::workbook::publication::open_publication_for_seal;
use census_store::Store;

use crate::census::{Verification, WorkbookCheck};

pub struct Inspection {
    pub dataset: ExportDataset,
    pub check: WorkbookCheck,
}

pub fn inspect_workbook(
    path: &Path,
    store: &Store,
    cohort: GradYear,
    scope: Scope,
    jurisdictions: &[census_domain::UsJurisdiction],
) -> Result<Inspection> {
    let lineage = store_lineage(store).context("measuring the store's current generation")?;
    let opened = open_publication_for_seal(path, &lineage, scope, cohort, jurisdictions)
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
            counts_reconciled: Verification::Verified,
            coverage_reconciled: Verification::Verified,
            metrics_reconciled: Verification::Verified,
            export_verified: Verification::Verified,
            discrepancies: Vec::new(),
        },
    })
}
