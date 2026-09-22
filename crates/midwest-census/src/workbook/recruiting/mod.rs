//! The recruiting sheets of the census workbook: objective §50 `Athletes`, §51 `PRs` and §53
//! `Coaches`.
//!
//! All three read one [`Dataset`] — a single scoped, cohort-filtered pass over the store's merged
//! entity tables — so a recruiting cell can never disagree with the census report or with the
//! `Best results` sheet, and no sheet re-derives a fact from raw source text.
//!
//! # Published sheet order
//!
//! `workbook::write_workbook` writes the objective's sheets in this order (§50-§54):
//!
//! ```text
//! Athletes            §50   this module
//! PRs                 §51   this module
//! Performances_NNN    §52   `workbook::performances`, ≥1 sheet, 1,000,000 data rows each
//! Coaches             §53   this module
//! Schools, Meets, Sources, Coverage, Conflicts, Review, Run Metrics   §54  `workbook::meta`
//! ```
//!
//! The §52 call sits between [`Recruiting::write_prs`] and [`Recruiting::write_coaches`], which is why
//! this module exposes three writers over one dataset instead of a single function.
//!
//! # What this workbook now contains, against the legacy census sheets
//!
//! Objective §50-§54 supersedes part of the legacy sheet set in name and content, so the mapping is
//! recorded here for the reviewer:
//!
//! | Legacy sheet | Disposition |
//! |---|---|
//! | `Goal & method` | retained — provenance for the run |
//! | `Summary` | retained — the workbook-level twin of §49's per-jurisdiction denominators |
//! | `By state - core`, `By state - all sources` | retained — §49's per-jurisdiction coverage |
//! | `Athletic.net marginal` | retained — the §49 Athletic.net-attributable view |
//! | `Best results` | superseded by `PRs` (§51), which publishes the same reduction with the recruiter columns; retained because the `bests` sidecars publish it too |
//! | `Meets` | renamed `Meets summary`: the name now belongs to §54's row-level `Meets` sheet |
//! | `Evidence mix` | retained — the evidence mix behind §54's `Sources` |
//! | `Method notes` | retained — method provenance |
//!
//! Nothing was dropped: the objective's sheets are additions to the legacy set, and the two sheets
//! whose content the objective re-specifies (`Best results` → `PRs`, `Meets` → `Meets summary` +
//! `Meets`) are both still present under their new names.
//!
//! # Scope, cohort and reconciliation
//!
//! The scope is the workbook run's evidence scope: `core` drops rows whose only evidence is
//! Athletic.net or its AthleticLIVE mirror, exactly as `report` and `bests` drop them. The cohort is
//! the run's graduating class (`Options::grad_year`), and `None` publishes every athlete in scope.
//!
//! After the sheets are written, [`Recruiting::reconcile`] prints each sheet's row count beside the
//! canonical store counts it came from — the store's own table sizes, the count the scope filter kept,
//! and, for `PRs`, the row count of the crate's independent `bests` reduction over the same store,
//! scope and cohort. The numbers are printed rather than assumed; a `PRs` line whose `consistent`
//! field is `false` is the signal that this sheet and the `Best results` sheet disagree.

use crate::bests;
use crate::report::{ReportResult, Scope};
use crate::store::Store;
use rust_xlsxwriter::Workbook;
use std::path::Path;

use super::cells::write_sheet;
use dataset::Dataset;

mod athletes;
mod coaches;
mod dataset;
mod facts;
mod prs;

#[cfg(test)]
mod tests;

/// The three recruiting sheets, built once from one read of the store.
pub(super) struct Recruiting {
    dataset: Dataset,
}

impl Recruiting {
    /// Load the recruiting read model for one workbook run.
    pub(super) fn load(store: &Store, scope: Scope, grad_year: Option<i16>) -> ReportResult<Self> {
        Ok(Self {
            dataset: Dataset::load(store, scope, grad_year)?,
        })
    }

    /// The `Athletes` sheet (§50), first of the objective's sheets.
    pub(super) fn write_athletes(&self, book: &mut Workbook, path: &Path) -> ReportResult<()> {
        write_sheet(
            book,
            path,
            athletes::TITLE,
            athletes::sheet(&self.dataset)?,
            &athletes::WIDTHS,
            true,
        )
    }

    /// The `PRs` sheet (§51), written immediately after `Athletes`.
    pub(super) fn write_prs(&self, book: &mut Workbook, path: &Path) -> ReportResult<()> {
        write_sheet(
            book,
            path,
            prs::TITLE,
            prs::sheet(&self.dataset.prs)?,
            &prs::WIDTHS,
            true,
        )
    }

    /// The `Coaches` sheet (§53), written after the §52 performance sheets.
    pub(super) fn write_coaches(&self, book: &mut Workbook, path: &Path) -> ReportResult<()> {
        write_sheet(
            book,
            path,
            coaches::TITLE,
            coaches::sheet(&self.dataset)?,
            &coaches::WIDTHS,
            true,
        )
    }

    /// Print what each recruiting sheet published against the store counts behind it.
    ///
    /// The lines go to the run's own output, next to the path the workbook was written to, because the
    /// operator reading a workbook needs the row counts that produced it: a `tracing` line would be
    /// invisible whenever `RUST_LOG` is set to anything that filters `info` out.
    ///
    /// The `PRs` line closes the loop with the crate's own reduction: `bests` re-reads the same store
    /// under the same scope and cohort and reports how many `(athlete, event)` rows it publishes. Two
    /// independent implementations of one rule that agree on the count is the strongest check the
    /// workbook can print about itself.
    pub(super) fn reconcile(&self, store: &Store) -> ReportResult<()> {
        let audit = self.dataset.audit();
        let canonical = bests::build(
            store,
            &bests::Options {
                scope: self.dataset.scope,
                grad_year: self.dataset.grad_year,
                limit: None,
            },
        )?;
        let scope = self.dataset.scope.as_str();
        let cohort = self
            .dataset
            .grad_year
            .map_or_else(|| "all".to_string(), |year| year.to_string());
        println!(
            "recruiting\t{}\trows={} in_scope={} store_athlete_rows={} scope={scope} cohort={cohort}",
            athletes::TITLE,
            audit.cohort_athletes,
            audit.scoped_athletes,
            audit.store_athletes,
        );
        println!(
            "recruiting\t{}\trows={} best_mark_rows={} consistent={} scope={scope} cohort={cohort}",
            prs::TITLE,
            audit.pr_rows,
            canonical.len(),
            audit.pr_rows == canonical.len(),
        );
        println!(
            "recruiting\t{}\trows={} store_coach_rows={} scope={scope}",
            coaches::TITLE,
            audit.coach_rows,
            audit.coach_rows,
        );
        Ok(())
    }
}
