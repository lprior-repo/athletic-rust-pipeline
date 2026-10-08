use std::path::Path;

use anyhow::Result;
use census_report::bests;
use census_report::export::ExportDataset;
use census_report::report::Scope;
use census_report::workbook;
use census_store::Store;

use super::{assertions, constants};

pub fn build_publication(
    store: &Store,
    root: &Path,
) -> Result<(
    workbook::publication::VerifiedPublication,
    super::artifacts::Semantics,
)> {
    let dataset = ExportDataset::load(store)?;

    let rows = bests::build_from_dataset(
        &dataset,
        &bests::Options {
            scope: Scope::Core,
            grad_year: Some(constants::COHORT),
            limit: None,
        },
    );
    assertions::assert_source_bests(&rows)?;

    let publication_root = root.join("publication");
    let written = workbook::build(
        store,
        &workbook::Options {
            grad_year: Some(constants::COHORT),
            out: Some(publication_root),
            limit: None,
            scope: Scope::Core,
            school_year: constants::SCHOOL_YEAR,
        },
    )?;
    let verified = workbook::publication::verify_published(&written)?;
    let artifacts = super::artifacts::read(&written)?;
    Ok((verified, artifacts))
}
