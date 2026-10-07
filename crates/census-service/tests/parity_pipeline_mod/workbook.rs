use std::path::Path;

use anyhow::Result;
use census_report::bests;
use census_report::export::ExportDataset;
use census_report::report::{self, Derivation, Scope};
use census_report::workbook;
use census_store::Store;

use super::{assertions, constants};

pub fn build_publication(
    store: &Store,
    root: &Path,
) -> Result<workbook::publication::VerifiedPublication> {
    let dataset = ExportDataset::load(store)?;
    let core = report::build_census(
        &Derivation::of(&dataset, Scope::Core, None),
        &store.out_dir(),
    );
    let all_sources = report::build_census(
        &Derivation::of(&dataset, Scope::AllSources, None),
        &store.out_dir(),
    );
    assertions::assert_scope_split(store, &core, &all_sources)?;

    let rows = bests::build_from_dataset(
        &dataset,
        &bests::Options {
            scope: Scope::Core,
            grad_year: Some(constants::COHORT),
            limit: None,
        },
    );
    assertions::assert_best_reduction(&rows, store, Scope::Core)?;

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
    Ok(workbook::publication::verify_published(&written)?)
}
