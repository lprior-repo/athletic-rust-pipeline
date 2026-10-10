mod spill_collapse;
mod spill_dir;
mod spill_merge;
mod spill_runs;

use super::join::PerformanceProjection;
pub(super) use super::rows::sheet_order;
use super::rows::PerformanceRow;
use crate::report::{Derivation, ReportError, ReportResult};
use std::path::{Path, PathBuf};

use spill_collapse::collapse_runs;
use spill_dir::SpillDir;
use spill_merge::RunMerge;
use spill_runs::spill_sorted_runs;

const RANGE_ROWS: u64 = 250_000;

const RUN_BYTES: u64 = 64 * 1024 * 1024;

const MAX_ROW_BYTES: u64 = 1024 * 1024;

const MAX_RANGES: usize = 64;

const MAX_MERGE_BYTES: u64 = 64 * 1024 * 1024;

const MAX_COLLAPSE_PASSES: usize = 32;

const COLLAPSE_FANIN_FLOOR: usize = 2;

pub(super) struct PerformanceRows {
    merge: RunMerge,
}

impl PerformanceRows {
    pub(super) fn build(derivation: &Derivation<'_>) -> ReportResult<Self> {
        Self::with_ranges(derivation, RANGE_ROWS, MAX_RANGES)
    }

    pub(super) fn with_ranges(
        derivation: &Derivation<'_>,
        range_rows: u64,
        max_ranges: usize,
    ) -> ReportResult<Self> {
        check_spill_budget(range_rows, max_ranges)?;
        let lookups = PerformanceProjection::of(derivation);
        let row_budget = spill_row_budget(derivation, range_rows, max_ranges)?;
        let dir = spill_dir::SpillDir::create()?;
        let projected = derivation
            .performances()
            .iter()
            .map(|performance| lookups.row(performance));
        let (rows, _) = Self::from_rows(dir, projected, row_budget, max_ranges)?;
        Ok(rows)
    }

    fn from_rows(
        dir: SpillDir,
        rows: impl Iterator<Item = PerformanceRow>,
        row_budget: u64,
        max_ranges: usize,
    ) -> ReportResult<(Self, Vec<RunMeta>)> {
        let spilled = spill_sorted_runs(dir.path(), rows, row_budget)?;
        let runs = collapse_runs(dir.path(), spilled, max_ranges)?;
        let files = run_paths(dir.path(), &runs)?;
        let merge = RunMerge::open(Some(dir), files)?;
        Ok((Self { merge }, runs))
    }
}

fn check_spill_budget(range_rows: u64, max_ranges: usize) -> ReportResult<()> {
    if range_rows == 0 {
        return Err(ReportError::Invariant {
            detail: "the performance spill was given zero rows per range".to_string(),
        });
    }
    if max_ranges == 0 {
        return Err(ReportError::Invariant {
            detail: "the performance spill was given no range to write".to_string(),
        });
    }
    if max_ranges > MAX_RANGES {
        return Err(ReportError::Invariant {
            detail: "the performance spill was given more ranges than it can merge".to_string(),
        });
    }
    Ok(())
}

fn spill_row_budget(
    derivation: &Derivation<'_>,
    range_rows: u64,
    max_ranges: usize,
) -> ReportResult<u64> {
    let held =
        u64::try_from(derivation.performances().len()).map_err(|_| ReportError::Invariant {
            detail: "the performance row count does not fit u64".to_string(),
        })?;
    rows_per_run(held, range_rows, max_ranges)
}

fn rows_per_run(total: u64, range_rows: u64, max_ranges: usize) -> ReportResult<u64> {
    let ranges = u64::try_from(max_ranges).map_err(|_| ReportError::Invariant {
        detail: "the wanted performance range count does not fit u64".to_string(),
    })?;
    Ok(range_rows.max(total.div_ceil(ranges.max(1))))
}

struct RunMeta {
    file: String,
    rows: u64,
    bytes: u64,
}

fn run_file(pass: usize, index: usize) -> String {
    format!("run-{pass:02}-{index:04}.jsonl")
}

fn run_paths(dir: &Path, runs: &[RunMeta]) -> ReportResult<Vec<(PathBuf, String)>> {
    let mut files = Vec::new();
    files
        .try_reserve(runs.len())
        .map_err(|_| ReportError::Invariant {
            detail: "the performance spill cannot open its sorted runs".to_string(),
        })?;
    for meta in runs {
        files.push((dir.join(&meta.file), meta.file.clone()));
    }
    Ok(files)
}

impl Iterator for PerformanceRows {
    type Item = ReportResult<PerformanceRow>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.merge.next_row() {
            Ok(row) => row.map(Ok),
            Err(error) => Some(Err(error)),
        }
    }
}

#[cfg(test)]
mod tests;
