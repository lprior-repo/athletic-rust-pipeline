use crate::export::public_identity::population::IdentityPopulation;
use crate::export::ExportDataset;
use crate::report::{Derivation, ReportResult};
use crate::workbook::Options;
use census_domain::model::SchoolYear;
use std::path::{Path, PathBuf};

pub(super) struct Inputs<'a> {
    pub(super) dataset: &'a ExportDataset,
    pub(super) identities: IdentityPopulation<'a>,
    pub(super) options: &'a Options,
    pub(super) derivation: Derivation<'a>,
    pub(super) admission: crate::report::EventAdmission<'a>,
    pub(super) school_year: SchoolYear,
    pub(super) out_root: PathBuf,
}

impl<'a> Inputs<'a> {
    pub(super) fn new(dataset: &'a ExportDataset, options: &'a Options) -> ReportResult<Self> {
        Self::with_derivation(
            dataset,
            options,
            Derivation::of(
                dataset,
                options.scope,
                options.grad_year,
                &options.jurisdictions,
            )?,
        )
    }

    fn with_derivation(
        dataset: &'a ExportDataset,
        options: &'a Options,
        derivation: Derivation<'a>,
    ) -> ReportResult<Self> {
        super::super::selected::ensure_verification_options(dataset, options)?;
        Ok(Self {
            dataset,
            options,
            derivation,
            admission: crate::report::EventAdmission::of(dataset)?,
            identities: IdentityPopulation::of(dataset)?,
            school_year: options.school_year,
            out_root: Path::new(&dataset.lineage.store_root).join("out"),
        })
    }

    pub(super) fn for_revision(
        dataset: &'a ExportDataset,
        options: &'a Options,
        revision: u32,
    ) -> ReportResult<Self> {
        if super::super::selected::binding(revision, dataset)?.is_none() {
            return Self::new(dataset, options);
        }
        let derivation = Derivation::selected(
            dataset,
            options.scope,
            options.grad_year,
            &options.jurisdictions,
            dataset.verified_current_affiliations()?,
        )?;
        Self::with_derivation(dataset, options, derivation)
    }

    pub(super) fn outcome(&self) -> ReportResult<crate::bests::SelectionOutcome> {
        if self.dataset.selected_profile_id().is_some() {
            return crate::bests::build_selected_outcome(
                &self.derivation,
                &self.admission,
                self.dataset.verified_participation()?,
                self.options.limit,
            );
        }
        crate::bests::build_admitted_outcome(
            &self.admission,
            &crate::bests::Options {
                scope: self.options.scope,
                grad_year: self.options.grad_year,
                limit: self.options.limit,
                jurisdictions: self.options.jurisdictions.clone(),
            },
        )
    }
}
