use crate::export::ExportDataset;
use crate::report::{Derivation, ReportResult};
use crate::workbook::Options;
use census_domain::model::SchoolYear;
use std::path::{Path, PathBuf};

pub(super) struct Inputs<'a> {
    pub(super) dataset: &'a ExportDataset,
    pub(super) options: &'a Options,
    pub(super) derivation: Derivation<'a>,
    pub(super) admission: crate::report::EventAdmission<'a>,
    pub(super) school_year: SchoolYear,
    pub(super) out_root: PathBuf,
}

impl<'a> Inputs<'a> {
    pub(super) fn new(dataset: &'a ExportDataset, options: &'a Options) -> ReportResult<Self> {
        let admission = crate::report::EventAdmission::of(dataset)?;
        let school_year = options.school_year;
        let out_root = Path::new(&dataset.lineage.store_root).join("out");
        Ok(Self {
            dataset,
            admission,
            options,
            derivation: Derivation::of(
                dataset,
                options.scope,
                options.grad_year,
                &options.jurisdictions,
            ),
            school_year,
            out_root,
        })
    }
}
