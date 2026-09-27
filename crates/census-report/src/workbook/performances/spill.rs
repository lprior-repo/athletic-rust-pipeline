
use super::join::{Lookups, Parents};
use super::rows::{sheet_order, PerformanceRow};
use crate::report::{io_error, retain_core_row, ReportError, ReportResult, Scope};
use census_store::{Store, Table};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

const RANGE_ROWS: u64 = 250_000;

const MAX_RANGES: usize = 256;

pub(super) struct PerformanceRows {
    dir: PathBuf,
    ranges: usize,
    next: usize,
    ready: std::vec::IntoIter<PerformanceRow>,
}

impl PerformanceRows {
    pub(super) fn build(store: &Store, scope: Scope, grad_year: Option<i16>) -> ReportResult<Self> {
        Self::with_ranges(store, scope, grad_year, RANGE_ROWS, MAX_RANGES)
    }

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

struct RangeFiles<'a> {
    dir: &'a Path,
    writers: &'a mut [BufWriter<File>],
    ranks: &'a HashMap<&'a str, usize>,
    names: usize,
}

impl RangeFiles<'_> {
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
        let files = self.writers.len().max(1);
        let ranges = rank.saturating_mul(files).checked_div(self.names.max(1));
        Ok(ranges.unwrap_or_default().min(files.saturating_sub(1)))
    }

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
        if let Err(error) = std::fs::remove_dir_all(&self.dir) {
            tracing::warn!(
                path = %self.dir.display(),
                %error,
                "could not remove the performance spill directory"
            );
        }
    }
}

fn ranges_for(rows: u64, range_rows: u64, max_ranges: usize, names: usize) -> usize {
    let wanted = rows.div_ceil(range_rows.max(1));
    let wanted = usize::try_from(wanted).unwrap_or(usize::MAX);
    wanted.clamp(1, max_ranges).min(names.max(1))
}

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
