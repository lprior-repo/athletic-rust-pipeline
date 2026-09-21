//! The census as one spreadsheet.
//!
//! Nine sheets, every cell copied from the typed census the crate already computes: the two scope
//! reports, the marginal view of what Athletic.net alone still contributes, the per-athlete best
//! marks, the meet inventory, the evidence mix, and the method notes. Nothing is recomputed here, so
//! the workbook can never disagree with `report`.
//!
//! The workbook is written with the same `rust_xlsxwriter` dependency the rest of the workspace uses;
//! there is no external script in the loop. `bests::write` sidecars are emitted alongside it, so the
//! best-mark reduction is readable as text too.

use crate::bests::{self, BestResult};
use crate::report::{build_census, Census, Scope};
use crate::store::Store;
use anyhow::{Context, Result};
use rust_xlsxwriter::Workbook;
use std::path::{Path, PathBuf};

mod cells;
mod inventory;
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
pub fn build(store: &Store, options: &Options) -> Result<PathBuf> {
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
    write_workbook(&path, &core, &all_sources, &bests, options.grad_year)?;
    Ok(path)
}

fn write_workbook(
    path: &Path,
    core: &Census,
    all_sources: &Census,
    bests: &[BestResult],
    grad_year: Option<i16>,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut book = Workbook::new();

    write_sheet(
        &mut book,
        "Goal & method",
        goal_sheet(core, grad_year),
        &[12, 96, 18, 12],
        false,
    )?;
    write_sheet(
        &mut book,
        "Summary",
        summary_sheet(core, all_sources)?,
        &[38, 22, 16, 14],
        false,
    )?;
    write_sheet(
        &mut book,
        "By state - core",
        state_sheet(core)?,
        &[
            10, 10, 12, 14, 11, 11, 18, 15, 14, 14, 17, 12, 16, 15, 17, 19,
        ],
        false,
    )?;
    write_sheet(
        &mut book,
        "By state - all sources",
        state_sheet(all_sources)?,
        &[
            10, 10, 12, 14, 11, 11, 18, 15, 14, 14, 17, 12, 16, 15, 17, 19,
        ],
        false,
    )?;
    write_sheet(
        &mut book,
        "Athletic.net marginal",
        marginal_sheet(core, all_sources)?,
        &[10, 16, 12, 30, 12, 16, 14],
        false,
    )?;
    write_sheet(
        &mut book,
        "Best results",
        best_sheet(bests)?,
        &[
            26, 14, 10, 9, 12, 10, 14, 16, 12, 13, 14, 10, 12, 11, 14, 26,
        ],
        true,
    )?;
    write_sheet(
        &mut book,
        "Meets",
        meets_sheet(core, all_sources)?,
        &[34, 12, 34, 12],
        false,
    )?;
    write_sheet(
        &mut book,
        "Evidence mix",
        evidence_sheet(all_sources)?,
        &[40, 12, 40, 12],
        false,
    )?;
    write_sheet(&mut book, "Method notes", method_sheet(), &[30, 110], false)?;

    book.save(path)
        .with_context(|| format!("saving the workbook to {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests;
