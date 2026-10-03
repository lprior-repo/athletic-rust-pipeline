use super::join::PerformanceProjection;
use super::rows::{sheet_order, PerformanceRow};
use crate::report::{io_error, Derivation, ReportError, ReportResult};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

const RANGE_ROWS: u64 = 250_000;

const MAX_RANGES: usize = 256;

pub(super) struct PerformanceRows {
    dir: SpillDir,
    ranges: usize,
    next: usize,
    ready: std::vec::IntoIter<PerformanceRow>,
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
        let lookups = PerformanceProjection::of(derivation);
        let (names, ranks) = bucket_universe(&lookups);
        let held =
            u64::try_from(derivation.performances().len()).map_err(|_| ReportError::Invariant {
                detail: "the performance row count does not fit u64".to_string(),
            })?;
        let ranges = ranges_for(held, range_rows, max_ranges, names.len())?;
        let dir = SpillDir::create()?;
        let mut writers = open_ranges(dir.path(), ranges)?;
        let mut files = RangeFiles {
            dir: dir.path(),
            writers: &mut writers,
            ranks: &ranks,
            names: names.len(),
        };
        spill(derivation, &lookups, &mut files)?;
        flush_ranges(dir.path(), &mut writers)?;
        Ok(Self {
            dir,
            ranges,
            next: 0,
            ready: Vec::new().into_iter(),
        })
    }
}

fn bucket_universe<'a>(
    lookups: &PerformanceProjection<'a>,
) -> (Vec<&'a str>, HashMap<&'a str, usize>) {
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
        Ok(ranges
            .map_or(Default::default(), core::convert::identity)
            .min(files.saturating_sub(1)))
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
    derivation: &Derivation<'_>,
    lookups: &PerformanceProjection<'_>,
    files: &mut RangeFiles<'_>,
) -> ReportResult<()> {
    for performance in derivation.performances() {
        let row = lookups.row(performance);
        files.write(&row)?;
    }
    Ok(())
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
            match read_range(self.dir.path(), range) {
                Ok(rows) => self.ready = rows.into_iter(),
                Err(error) => return Some(Err(error)),
            }
        }
    }
}

struct SpillDir {
    path: PathBuf,
}

impl SpillDir {
    fn create() -> ReportResult<Self> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|_| ReportError::Invariant {
                detail: "the system clock reads before the Unix epoch, so the performance spill \
                         directory has no unique name"
                    .to_string(),
            })?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "census-performance-rows-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).map_err(|source| io_error(&path, source))?;
        Ok(Self { path })
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for SpillDir {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.path) {
            tracing::warn!(
                path = %self.path.display(),
                %error,
                "could not remove the performance spill directory"
            );
        }
    }
}

fn ranges_for(rows: u64, range_rows: u64, max_ranges: usize, names: usize) -> ReportResult<usize> {
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
    let wanted = rows.div_ceil(range_rows);
    let wanted = usize::try_from(wanted).map_err(|_| ReportError::Invariant {
        detail: "the wanted performance range count does not fit usize".to_string(),
    })?;
    Ok(wanted.max(1).min(max_ranges).min(names.max(1)))
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
