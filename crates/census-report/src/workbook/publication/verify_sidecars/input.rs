use crate::export::ExportDataset;
use crate::report::{Derivation, ReportResult};
use crate::workbook::Options;
use census_domain::model::SchoolYear;
use std::path::{Path, PathBuf};

use super::defect;

pub(super) struct Inputs<'a> {
    pub(super) dataset: &'a ExportDataset,
    pub(super) options: &'a Options,
    pub(super) derivation: Derivation<'a>,
    pub(super) school_year: SchoolYear,
    pub(super) out_root: PathBuf,
}

impl<'a> Inputs<'a> {
    pub(super) fn new(dataset: &'a ExportDataset, options: &'a Options) -> ReportResult<Self> {
        let school_year = options
            .school_year
            .or_else(|| SchoolYear::from_date(&dataset.lineage.generated_on))
            .ok_or_else(|| {
                defect(format!(
                    "cannot determine the contact school year from {}",
                    dataset.lineage.generated_on
                ))
            })?;
        let out_root = Path::new(&dataset.lineage.store_root).join("out");
        Ok(Self {
            dataset,
            options,
            derivation: Derivation::of(dataset, options.scope, options.grad_year),
            school_year,
            out_root,
        })
    }
}
