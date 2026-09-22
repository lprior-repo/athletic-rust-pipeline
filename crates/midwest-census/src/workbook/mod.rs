//! The census as one spreadsheet: the objective's recruiting workbook, then the census provenance
//! sheets the platform has always published.
//!
//! # What the workbook contains, in the order it is written
//!
//! Objective §50-§54 sheets first, in the objective's own order:
//!
//! ```text
//! Athletes          §50   workbook::recruiting   one row per canonical athlete in the cohort
//! PRs               §51   workbook::recruiting   one row per athlete/event
//! Performances_00N  §52   workbook::performances every stored mark, partitioned to Excel's cap
//! Coaches           §53   workbook::recruiting   one row per canonical coach
//! Schools, Meets, Sources, Coverage, Conflicts, Review, Run Metrics  §54  workbook::meta
//! ```
//!
//! Then the retained legacy census sheets, unchanged and in their original order: `Goal & method`,
//! `Summary`, `By state - core`, `By state - all sources`, `Athletic.net marginal`, `Best results`,
//! `Meets summary`, `Evidence mix`, `Method notes`. `workbook::recruiting`'s module documentation holds
//! the disposition table for these — which the objective supersedes, which it merely renames
//! (`Meets` → `Meets summary`, because §54's row-level `Meets` sheet now owns that name), and which are
//! retained as provenance.
//!
//! Every cell is copied from the typed store the crate already computes or from a documented rule over
//! it: the recruiting sheets read the store's merged entity tables through one scoped pass
//! (`workbook::recruiting::dataset`), the census sheets read `report`, and `bests` supplies the
//! best-mark reduction both the `Best results` and `PRs` sheets rest on. Nothing is recomputed from raw
//! source text, so the workbook can never disagree with `report`.
//!
//! The workbook is written with the same `rust_xlsxwriter` dependency the rest of the workspace uses;
//! there is no external script in the loop. `bests::write` sidecars are emitted alongside it, so the
//! best-mark reduction is readable as text too.

use crate::bests::{self, BestResult};
use crate::report::{build_census, io_error, xlsx_error, Census, ReportResult, Scope};
use crate::store::Store;
use rust_xlsxwriter::Workbook;
use std::path::{Path, PathBuf};

mod cells;
mod inventory;
mod meta;

pub(crate) use meta::retained_records;
mod performances;
mod recruiting;
mod sheets;

use cells::write_sheet;
use inventory::{best_sheet, evidence_sheet, meets_sheet, method_sheet};
use sheets::{goal_sheet, marginal_sheet, state_sheet, summary_sheet};

#[derive(Debug, Clone)]
pub struct Options {
    pub grad_year: Option<i16>,
    pub out: Option<PathBuf>,
    pub limit: Option<usize>,
    /// Evidence scope the best-results sheet and its `bests::write` sidecars are reduced over. The
    /// workbook always publishes both census scopes, but there is one best-results reduction, and
    /// this is it.
    pub scope: Scope,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            grad_year: Some(2027),
            out: None,
            limit: None,
            scope: Scope::Core,
        }
    }
}

/// Build the workbook and its text sidecars; returns the path of the `.xlsx`.
pub fn build(store: &Store, options: &Options) -> ReportResult<PathBuf> {
    let core = build_census(store, Scope::Core)?;
    let all_sources = build_census(store, Scope::AllSources)?;
    let bests = bests::build(
        store,
        &bests::Options {
            scope: options.scope,
            grad_year: options.grad_year,
            limit: options.limit,
        },
    )?;
    let cohort = options
        .grad_year
        .map(|year| format!("co{year}"))
        .unwrap_or_else(|| "all".to_string());
    bests::write(store, &bests, &cohort)?;

    let path = options.out.clone().unwrap_or_else(|| {
        store
            .out_dir()
            .join(format!("midwest-census-{}.xlsx", core.generated_on))
    });
    write_workbook(
        &path,
        store,
        &core,
        &all_sources,
        &bests,
        options.grad_year,
        options.scope,
    )?;
    Ok(path)
}

/// The three census outputs the sheet blocks read: the two evidence scopes and the best-mark
/// reduction the `Best results` and `PRs` sheets rest on.
#[derive(Debug, Clone, Copy)]
struct Views<'a> {
    core: &'a Census,
    all_sources: &'a Census,
    bests: &'a [BestResult],
}

/// Write every sheet, objective sheets first and the legacy census sheets after them.
fn write_workbook(
    path: &Path,
    store: &Store,
    core: &Census,
    all_sources: &Census,
    bests: &[BestResult],
    grad_year: Option<i16>,
    scope: Scope,
) -> ReportResult<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
    }
    let mut book = Workbook::new();
    let views = Views {
        core,
        all_sources,
        bests,
    };

    write_objective_sheets(&mut book, path, store, views, scope, grad_year)?;
    write_census_sheets(&mut book, path, views, grad_year)?;

    book.save(path).map_err(|source| xlsx_error(path, source))?;
    Ok(())
}

/// The objective's §50-§54 sheets, in the objective's order.
///
/// The recruiting sheets are one dataset read three times around the §52 performance sheets: `Athletes`
/// and `PRs` first, `Coaches` after them, which is the order §50-§53 publishes, and §54's operational
/// sheets close the objective block. The §51 `PRs` sheet and the `PRs`-scope best-mark reduction are
/// reconciled against each other after every sheet is written, so the run's log carries the row counts
/// rather than an assumption about them.
fn write_objective_sheets(
    book: &mut Workbook,
    path: &Path,
    store: &Store,
    views: Views<'_>,
    scope: Scope,
    grad_year: Option<i16>,
) -> ReportResult<()> {
    let recruiting = recruiting::Recruiting::load(store, scope, grad_year)?;
    recruiting.write_athletes(book, path)?;
    recruiting.write_prs(book, path)?;
    performances::write_performance_sheets(book, path, store, scope)?;
    recruiting.write_coaches(book, path)?;
    recruiting.reconcile(store)?;
    meta::write_meta_sheets(
        book,
        path,
        store,
        views.core,
        views.all_sources,
        views.bests,
    )
}

/// The retained legacy census sheets, in the order the workbook has always written them.
fn write_census_sheets(
    book: &mut Workbook,
    path: &Path,
    views: Views<'_>,
    grad_year: Option<i16>,
) -> ReportResult<()> {
    let core = views.core;
    let all_sources = views.all_sources;
    write_sheet(
        book,
        path,
        "Goal & method",
        goal_sheet(core, grad_year),
        &[12, 96, 18, 12],
        false,
    )?;
    write_sheet(
        book,
        path,
        "Summary",
        summary_sheet(core, all_sources)?,
        &[38, 22, 16, 14],
        false,
    )?;
    write_state_sheets(book, path, core, all_sources)?;
    write_artifact_sheets(book, path, core, all_sources, views.bests)
}

/// Both per-state views, in published sheet order: core first, then every source.
fn write_state_sheets(
    book: &mut Workbook,
    path: &Path,
    core: &Census,
    all_sources: &Census,
) -> ReportResult<()> {
    let widths = [
        10, 10, 12, 14, 11, 11, 18, 15, 14, 14, 17, 12, 16, 15, 17, 19,
    ];
    for (name, census) in [
        ("By state - core", core),
        ("By state - all sources", all_sources),
    ] {
        write_sheet(book, path, name, state_sheet(census)?, &widths, false)?;
    }
    Ok(())
}

/// The marginal, best-results, meet, evidence and method sheets, in published sheet order.
fn write_artifact_sheets(
    book: &mut Workbook,
    path: &Path,
    core: &Census,
    all_sources: &Census,
    bests: &[BestResult],
) -> ReportResult<()> {
    write_sheet(
        book,
        path,
        "Athletic.net marginal",
        marginal_sheet(core, all_sources)?,
        &[10, 16, 12, 30, 12, 16, 14],
        false,
    )?;
    write_sheet(
        book,
        path,
        "Best results",
        best_sheet(bests)?,
        &[
            26, 14, 10, 9, 12, 10, 14, 16, 12, 13, 14, 10, 12, 11, 14, 26,
        ],
        true,
    )?;
    write_sheet(
        book,
        path,
        "Meets summary",
        meets_sheet(core, all_sources)?,
        &[34, 12, 34, 12],
        false,
    )?;
    write_sheet(
        book,
        path,
        "Evidence mix",
        evidence_sheet(all_sources)?,
        &[40, 12, 40, 12],
        false,
    )?;
    write_sheet(
        book,
        path,
        "Method notes",
        method_sheet(),
        &[30, 110],
        false,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests;
