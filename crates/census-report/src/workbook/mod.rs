//! The census as one spreadsheet: the recruiting workbook, then the census provenance
//! sheets the platform publishes.
//!
//! # What the workbook contains, in the order it is written
//!
//! ```text
//! Athletes          §50   one row per canonical athlete in the cohort
//! PRs               §51   one row per athlete/event
//! Performances_00N  §52   every stored mark, partitioned to Excel's cap
//! Coaches           §53   one row per canonical coach
//! Schools           §54   one row per canonical school the run's scope retains
//! Meets             §54   one row per canonical meet
//! Sources           §54   source declarations and evidence
//! Coverage          §54   jurisdiction coverage and gaps
//! Conflicts         §54   the rows the merge kept separate instead of resolving
//! Review            §54   the retained review rows and durable model verdicts
//! Run Metrics       §54   run counters and reconciliation
//! ```
//!
//! The superseded legacy views were dropped because their numbers are published by `Coverage`,
//! `Sources`, `Run Metrics` and `PRs`.
//!
//! Every cell is copied from the typed store the crate already computes or from a documented rule over
//! it: the recruiting sheets read the store's merged entity tables through one scoped pass
//! (`workbook::recruiting::dataset`), the census sheets read `report`, and `bests` supplies the
//! best-mark reduction for the `PRs` sheet and text sidecars. Nothing is recomputed from raw source
//! text, so the workbook can never disagree with `report`.
//!
//! The workbook is written with the same `rust_xlsxwriter` dependency the rest of the workspace uses;
//! there is no external script in the loop. `bests::write` sidecars are emitted alongside it, so the
//! best-mark reduction is readable as text too.
use crate::bests::{self, BestResult};
use crate::report::{build_census, io_error, xlsx_error, Census, ReportResult, Scope};
use census_store::Store;
use rust_xlsxwriter::Workbook;
use std::path::{Path, PathBuf};

mod cells;
pub mod meta;

pub use meta::retained_records;
mod performances;
mod recruiting;

#[derive(Debug, Clone)]
pub struct Options {
    pub grad_year: Option<i16>,
    pub out: Option<PathBuf>,
    pub limit: Option<usize>,
    /// Evidence scope for the best-mark reduction and its `bests::write` sidecars. The workbook
    /// publishes both census scopes, but there is one best-mark reduction, and this is it.
    pub scope: Scope,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            grad_year: Some(2027),
            out: None,
            limit: None,
            scope: Scope::AllSources,
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
            .join(format!("census-service-{}.xlsx", core.generated_on))
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
/// reduction the `PRs` sheet and text sidecars rest on.
#[derive(Debug, Clone, Copy)]
struct Views<'a> {
    core: &'a Census,
    all_sources: &'a Census,
    bests: &'a [BestResult],
}

/// Write every sheet in the frozen publish order.
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
    let perf_population =
        performances::write_performance_sheets(book, path, store, scope, grad_year)?;
    let perf_population = performances::PerformanceSheetPopulation {
        cohort_year: perf_population.cohort_year,
        scope: perf_population.scope,
        cohort_athletes: recruiting.cohort_athletes(),
        total_rows: perf_population.total_rows,
    };
    recruiting.write_coaches(book, path)?;
    recruiting.reconcile(store)?;
    meta::write_meta_sheets(
        book,
        path,
        meta::RunFacts {
            store,
            core: views.core,
            all_sources: views.all_sources,
            bests: views.bests,
            scope,
            perf_population,
        },
    )
}
