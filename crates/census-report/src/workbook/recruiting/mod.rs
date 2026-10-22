use crate::bests;
use crate::report::{Derivation, ReportResult};
use rust_xlsxwriter::Workbook;
use std::path::Path;

use super::cells::{write_sheet_rows, SheetChrome, SheetLayout};
use dataset::Dataset;

mod athletes;
pub(in crate::workbook) mod bounded;
pub(in crate::workbook) mod coach_projection;
pub(in crate::workbook) mod coach_spelling;
mod coaches;
#[cfg(test)]
pub(in crate::workbook) mod coalesce;
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

pub(super) struct Recruiting<'a> {
    dataset: Dataset<'a>,
    reported: Option<crate::workbook::reported::Projection<'a>>,
}

impl<'a> Recruiting<'a> {
    pub(super) fn of<'d>(
        derivation: &'a Derivation<'d>,
        school_year: census_domain::model::SchoolYear,
        prs: Vec<bests::SharedSelection>,
        admission: &crate::report::EventAdmission<'d>,
    ) -> ReportResult<Self> {
        let reported = crate::workbook::reported::Projection::of(
            derivation.dataset(),
            derivation,
            admission,
            &prs,
        )?;
        Ok(Self {
            dataset: Dataset::of(derivation, school_year, prs, admission)?,
            reported,
        })
    }

    pub(super) fn write_athletes(&self, book: &mut Workbook, path: &Path) -> ReportResult<()> {
        let mut widths = self.identity_widths(&athletes::WIDTHS);
        if self.reported.is_some() {
            widths.splice(0..0, [24, 36]);
            widths.extend([48, 60, 22, 14, 32]);
        }
        write_sheet_rows(
            book,
            path,
            SheetLayout {
                name: athletes::TITLE,
                widths: &widths,
                chrome: SheetChrome::Filtered,
            },
            athletes::rows_with_reported(&self.dataset, self.reported.as_ref()),
        )
    }

    pub(super) fn write_prs(&self, book: &mut Workbook, path: &Path) -> ReportResult<()> {
        let schools = if self.dataset.source.selected_profile_id().is_some() {
            Some(crate::export::current_school::SelectedSchools::of(
                self.dataset.source,
            )?)
        } else {
            None
        };
        let mut widths = self.identity_widths(&prs::WIDTHS);
        if schools.is_some() {
            widths.extend([30, 14, 22]);
        }
        if self.reported.is_some() {
            widths.splice(0..0, [24, 40]);
            widths.extend([22, 60]);
        }
        write_sheet_rows(
            book,
            path,
            SheetLayout {
                name: prs::TITLE,
                widths: &widths,
                chrome: SheetChrome::Filtered,
            },
            prs::rows_with_reported(
                &self.dataset.prs,
                self.dataset.public_identities,
                schools.as_ref(),
                self.reported.as_ref(),
            ),
        )
    }

    fn identity_widths(&self, widths: &[u16]) -> Vec<u16> {
        widths
            .iter()
            .copied()
            .chain(
                self.dataset
                    .public_identities
                    .into_iter()
                    .flat_map(|_| [24, 60]),
            )
            .collect()
    }

    pub(super) fn write_coaches(&self, book: &mut Workbook, path: &Path) -> ReportResult<()> {
        write_sheet_rows(
            book,
            path,
            SheetLayout {
                name: coaches::TITLE,
                widths: &coaches::WIDTHS,
                chrome: SheetChrome::Filtered,
            },
            coaches::rows(&self.dataset),
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
            published_coach_rows = audit.published_coach_rows,
            contact_conflicts = audit.contact_conflicts,
            "recruiting projection counts"
        );
    }
}
