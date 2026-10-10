use crate::bests;
use crate::report::{Derivation, ReportResult};
use rust_xlsxwriter::Workbook;
use std::path::Path;

use super::cells::{write_sheet, SheetLayout};
use dataset::Dataset;

mod athletes;
pub(in crate::workbook) mod coach_projection;
pub(in crate::workbook) mod coach_spelling;
mod coaches;
mod columns;
pub(in crate::workbook) mod contact;
mod csv;
mod dataset;
mod facts;
pub(in crate::workbook) mod mailbox_provenance;
pub(in crate::workbook) mod profiles;
mod prs;

#[cfg(test)]
mod provenance_tests;
#[cfg(test)]
mod tests;

pub(in crate::workbook) use contact::{disagreements, Disagreement};
pub use csv::{write_recruiting_csv, RecruitingCsvCounts};

pub(super) struct Recruiting {
    dataset: Dataset,
}

impl Recruiting {
    pub(super) fn of(
        derivation: &Derivation<'_>,
        school_year: census_domain::model::SchoolYear,
        prs: Vec<bests::SharedSelection>,
    ) -> ReportResult<Self> {
        Ok(Self {
            dataset: Dataset::of(derivation, school_year, prs)?,
        })
    }

    pub(super) fn write_athletes(&self, book: &mut Workbook, path: &Path) -> ReportResult<()> {
        write_sheet(
            book,
            path,
            SheetLayout {
                name: athletes::TITLE,
                widths: &athletes::WIDTHS,
                autofilter: true,
            },
            athletes::sheet(&self.dataset)?,
        )
    }

    pub(super) fn write_prs(&self, book: &mut Workbook, path: &Path) -> ReportResult<()> {
        write_sheet(
            book,
            path,
            SheetLayout {
                name: prs::TITLE,
                widths: &prs::WIDTHS,
                autofilter: true,
            },
            prs::sheet(&self.dataset.prs)?,
        )
    }

    pub(super) fn write_coaches(&self, book: &mut Workbook, path: &Path) -> ReportResult<()> {
        write_sheet(
            book,
            path,
            SheetLayout {
                name: coaches::TITLE,
                widths: &coaches::WIDTHS,
                autofilter: true,
            },
            coaches::sheet(&self.dataset)?,
        )
    }

    pub(super) fn selected_prs(&self) -> &[bests::SharedSelection] {
        &self.dataset.prs
    }

    pub(super) fn trace_counts(&self) {
        let audit = self.dataset.audit();
        tracing::info!(
            scope = self.dataset.scope.as_str(),
            cohort = ?self.dataset.grad_year,
            store_athletes = audit.store_athletes,
            scoped_athletes = audit.scoped_athletes,
            cohort_athletes = audit.cohort_athletes,
            pr_rows = audit.pr_rows,
            coach_rows = audit.coach_rows,
            contact_conflicts = audit.contact_conflicts,
            "recruiting projection counts"
        );
    }
}
