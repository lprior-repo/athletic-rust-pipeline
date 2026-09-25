//! Handing the §52 rows over one school-name range at a time, sorted, without holding the table.
//!
//! [`sheet_order`] starts with the school name, so the rows for one school are contiguous, and so are
//! the rows for a contiguous slice of the school-name universe. That is the seam this module cuts
//! along: [`PerformanceRows::build`] reads the performance table once, joins each row, and spills it
//! into the range file whose slice of names it falls in; the ranges are then read back in name order,
//! sorted, and handed out one at a time. Nothing larger than one range is ever resident — the parent
//! tables the join reads are the other resident set, and they are held indexed, whole, for as long as
//! the spill is being written.
//!
//! The spill is a directory under the system temp dir, removed when the value drops. A process that
//! dies mid-pass leaves it behind; it is inert, since nothing reads it unless a `PerformanceRows`
//! names it, and the name carries this process id.

use super::join::{Lookups, Parents};
use super::rows::{sheet_order, PerformanceRow};
use crate::report::{io_error, retain_core_row, ReportError, ReportResult, Scope};
use census_store::{Store, Table};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

/// Rows one range holds. Each range is sorted on its own, so this is the sort's memory unit: a
/// quarter million rows of JSONL at the census's row width are a few hundred megabytes at most.
const RANGE_ROWS: u64 = 250_000;

/// Ranges at most, hence files open at once while the spill is written.
const MAX_RANGES: usize = 256;

/// The §52 rows in sheet order, produced one school-name range at a time.
pub(super) struct PerformanceRows {
    dir: PathBuf,
    ranges: usize,
    next: usize,
    ready: std::vec::IntoIter<PerformanceRow>,
}

impl PerformanceRows {
    /// Build the rows for `store` under `scope`, restricted to the cohort.
    pub(super) fn build(store: &Store, scope: Scope, grad_year: Option<i16>) -> ReportResult<Self> {
        Self::with_ranges(store, scope, grad_year, RANGE_ROWS, MAX_RANGES)
    }

    /// The same, with the range sizing a caller chooses — so a test can prove the range seams without
    /// a quarter-million rows.
    pub(super) fn with_ranges(
        store: &Store,
        scope: Scope,
        grad_year: Option<i16>,
        range_rows: u64,
        max_ranges: usize,
    ) -> ReportResult<Self> {
        let parents = Parents::read(store, scope, grad_year)?;
        let lookups = parents.lookups();
        let (names, ranks) = bucket_universe(&lookups);
        let cohort = parents.cohort_set();
        let ranges = ranges_for(held_rows(store)?, range_rows, max_ranges, names.len());
        let dir = spill_dir()?;
        let mut writers = open_ranges(&dir, ranges)?;
        let mut files = RangeFiles {
            dir: &dir,
            writers: &mut writers,
            ranks: &ranks,
            names: names.len(),
        };
        spill(store, scope, &lookups, cohort, &mut files)?;
        flush_ranges(&dir, &mut writers)?;
        Ok(Self {
            dir,
            ranges,
            next: 0,
            ready: Vec::new().into_iter(),
        })
    }
}

/// The bucket universe: the names a row can print, plus the empty name an unresolved school prints,
/// sorted so a contiguous slice of ranks is a contiguous slice of the sheet order — with the rank of
/// each name.
fn bucket_universe<'a>(lookups: &Lookups<'a>) -> (Vec<&'a str>, HashMap<&'a str, usize>) {
    let mut names: Vec<&str> = lookups.school_names();
    names.push("");
    names.sort_unstable();
    names.dedup();
    let ranks = names
        .iter()
        .enumerate()
        .map(|(rank, name)| (*name, rank))
        .collect();
    (names, ranks)
}

/// The performance table's observation count: every version counted, so it can only over-estimate the
/// merged rows, and an over-estimate buys more ranges, never larger ones.
fn held_rows(store: &Store) -> ReportResult<u64> {
    let count = store
        .stats()?
        .tables
        .iter()
        .find(|(table, _)| table == Table::Performances.file())
        .map(|(_, count)| *count)
        .unwrap_or(0);
    Ok(count)
}

/// The range files, and what places a row among them.
struct RangeFiles<'a> {
    dir: &'a Path,
    writers: &'a mut [BufWriter<File>],
    ranks: &'a HashMap<&'a str, usize>,
    names: usize,
}

impl RangeFiles<'_> {
    /// The range a row's school name falls in, or the invariant that says why it cannot be placed.
    fn range_of(&self, row: &PerformanceRow) -> ReportResult<usize> {
        let Some(rank) = self.ranks.get(row.school.as_str()).copied() else {
            return Err(ReportError::Invariant {
                detail: format!(
                    "performance {} prints school {:?}, which the in-scope school set does not \
                     hold; the row cannot be placed in sheet order",
                    row.id, row.school
                ),
            });
        };
        // The universe always holds at least the empty name and the spill always opens at least one
        // range file, so neither the product nor the division can leave the file list's bounds.
        let files = self.writers.len().max(1);
        let ranges = rank.saturating_mul(files).checked_div(self.names.max(1));
        Ok(ranges.unwrap_or_default().min(files.saturating_sub(1)))
    }

    /// Spill one joined row into the range file its range names.
    fn write(&mut self, row: &PerformanceRow) -> ReportResult<()> {
        let range = self.range_of(row)?;
        let Some(writer) = self.writers.get_mut(range) else {
            return Err(ReportError::Invariant {
                detail: format!(
                    "performance {} answers range {range}, which the spill opened no file for",
                    row.id
                ),
            });
        };
        write_row(writer, row).map_err(|source| io_error(&range_path(self.dir, range), source))
    }
}

/// Read the performance table once, joining every row `scope` keeps and spilling it.
fn spill(
    store: &Store,
    scope: Scope,
    lookups: &Lookups<'_>,
    cohort: HashSet<&str>,
    files: &mut RangeFiles<'_>,
) -> ReportResult<()> {
    let mut failure: Option<ReportError> = None;
    store.for_each_merged(
        Table::Performances,
        |mut performance: census_domain::model::CanonicalPerformance| {
            if failure.is_some() {
                // The pass is over as far as this caller is concerned; the refusal is returned below, and
                // reading further rows would only spend time on an answer nobody wants.
                return Ok(());
            }
            if scope == Scope::Core && !retain_core_row(&mut performance) {
                return Ok(());
            }
            if !cohort.contains(performance.athlete.as_str()) {
                return Ok(());
            }
            let row = lookups.row(&performance);
            if let Err(error) = files.write(&row) {
                failure = Some(error);
            }
            Ok(())
        },
    )?;
    match failure {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

/// Flush every range file, naming the file a refusal came from.
fn flush_ranges(dir: &Path, writers: &mut [BufWriter<File>]) -> ReportResult<()> {
    for (index, writer) in writers.iter_mut().enumerate() {
        if let Err(source) = writer.flush() {
            return Err(io_error(&range_path(dir, index), source));
        }
    }
    Ok(())
}

impl Iterator for PerformanceRows {
    type Item = ReportResult<PerformanceRow>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(row) = self.ready.next() {
                return Some(Ok(row));
            }
            if self.next >= self.ranges {
                return None;
            }
            let range = self.next;
            self.next = self.next.saturating_add(1);
            match read_range(&self.dir, range) {
                Ok(rows) => self.ready = rows.into_iter(),
                Err(error) => return Some(Err(error)),
            }
        }
    }
}

impl Drop for PerformanceRows {
    fn drop(&mut self) {
        // The rows are the workbook's by then, and the spill has no other reader. A destructor cannot
        // report a failure, so a directory that will not go is named and left for the temp cleaner.
        if let Err(error) = std::fs::remove_dir_all(&self.dir) {
            tracing::warn!(
                path = %self.dir.display(),
                %error,
                "could not remove the performance spill directory"
            );
        }
    }
}

/// The ranges a table of `rows` rows is split into: enough that one range is a bounded sort, and
/// never more than there are names to order.
fn ranges_for(rows: u64, range_rows: u64, max_ranges: usize, names: usize) -> usize {
    let wanted = rows.div_ceil(range_rows.max(1));
    let wanted = usize::try_from(wanted).unwrap_or(usize::MAX);
    wanted.clamp(1, max_ranges).min(names.max(1))
}

/// A directory under the system temp dir, named for this process so two runs cannot share one.
fn spill_dir() -> ReportResult<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or(0);
    let dir = std::env::temp_dir().join(format!(
        "census-performance-rows-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).map_err(|source| io_error(&dir, source))?;
    Ok(dir)
}

fn range_path(dir: &Path, range: usize) -> PathBuf {
    dir.join(format!("range-{range:04}.jsonl"))
}

fn open_ranges(dir: &Path, ranges: usize) -> ReportResult<Vec<BufWriter<File>>> {
    (0..ranges)
        .map(|range| {
            let path = range_path(dir, range);
            File::create(&path)
                .map(BufWriter::new)
                .map_err(|source| io_error(&path, source))
        })
        .collect()
}

fn write_row(writer: &mut BufWriter<File>, row: &PerformanceRow) -> std::io::Result<()> {
    serde_json::to_writer(&mut *writer, row)?;
    writer.write_all(b"\n")
}

/// One range's rows, sorted into sheet order.
fn read_range(dir: &Path, range: usize) -> ReportResult<Vec<PerformanceRow>> {
    let path = range_path(dir, range);
    let file = File::open(&path).map_err(|source| io_error(&path, source))?;
    let mut rows = Vec::new();
    for (line, entry) in BufReader::new(file).lines().enumerate() {
        let entry = entry.map_err(|source| io_error(&path, source))?;
        if entry.is_empty() {
            continue;
        }
        rows.push(
            serde_json::from_str(&entry).map_err(|source| ReportError::Decode {
                path: path.clone(),
                line: line.saturating_add(1),
                source,
            })?,
        );
    }
    rows.sort_by(sheet_order);
    Ok(rows)
}

#[cfg(test)]
mod tests;
