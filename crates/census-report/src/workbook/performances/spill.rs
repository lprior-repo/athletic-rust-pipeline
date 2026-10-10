use super::join::PerformanceProjection;
use super::rows::{sheet_order, PerformanceRow};
use crate::report::{io_error, Derivation, ReportError, ReportResult};
use std::collections::BinaryHeap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

const RANGE_ROWS: u64 = 250_000;

const RUN_BYTES: u64 = 64 * 1024 * 1024;

const MAX_ROW_BYTES: u64 = 1024 * 1024;

const MAX_RANGES: usize = 256;

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
        let lookups = PerformanceProjection::of(derivation);
        let held =
            u64::try_from(derivation.performances().len()).map_err(|_| ReportError::Invariant {
                detail: "the performance row count does not fit u64".to_string(),
            })?;
        let row_budget = rows_per_run(held, range_rows, max_ranges)?;
        let dir = SpillDir::create()?;
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

fn spill_sorted_runs(
    dir: &Path,
    rows: impl Iterator<Item = PerformanceRow>,
    row_budget: u64,
) -> ReportResult<Vec<RunMeta>> {
    let mut spilled = Vec::new();
    let mut run = Vec::new();
    let mut run_bytes: u64 = 0;
    let mut index: usize = 0;
    for row in rows {
        let row_bytes = encoded_row_bytes(&row)?;
        if row_bytes > MAX_ROW_BYTES {
            return Err(ReportError::Invariant {
                detail: format!("performance {} exceeds the spill row budget", row.id),
            });
        }
        if run_is_full(&run, run_bytes, row_budget, row_bytes)? {
            flush_run(dir, &mut run, run_bytes, &mut index, &mut spilled)?;
            run_bytes = 0;
        }
        run.try_reserve(1).map_err(|_| ReportError::Invariant {
            detail: "the performance spill cannot grow its sorted run".to_string(),
        })?;
        run.push(row);
        run_bytes = run_bytes
            .checked_add(row_bytes)
            .ok_or_else(|| ReportError::Invariant {
                detail: "the performance spill cannot count its run bytes".to_string(),
            })?;
    }
    if !run.is_empty() {
        flush_run(dir, &mut run, run_bytes, &mut index, &mut spilled)?;
    }
    Ok(spilled)
}

fn encoded_row_bytes(row: &PerformanceRow) -> ReportResult<u64> {
    let encoded = serde_json::to_vec(row).map_err(|_| ReportError::Invariant {
        detail: "a performance row does not serialize".to_string(),
    })?;
    let bytes = u64::try_from(encoded.len()).map_err(|_| ReportError::Invariant {
        detail: "a performance row does not fit its byte count".to_string(),
    })?;
    bytes.checked_add(1).ok_or_else(|| ReportError::Invariant {
        detail: "a performance row does not fit its byte count".to_string(),
    })
}

fn run_is_full(
    run: &[PerformanceRow],
    run_bytes: u64,
    row_budget: u64,
    row_bytes: u64,
) -> ReportResult<bool> {
    if run.is_empty() {
        return Ok(false);
    }
    let rows = u64::try_from(run.len()).map_err(|_| ReportError::Invariant {
        detail: "a sorted run does not fit its row count".to_string(),
    })?;
    if rows >= row_budget {
        return Ok(true);
    }
    Ok(run_bytes
        .checked_add(row_bytes)
        .is_none_or(|total| total > RUN_BYTES))
}

fn flush_run(
    dir: &Path,
    run: &mut Vec<PerformanceRow>,
    bytes: u64,
    index: &mut usize,
    spilled: &mut Vec<RunMeta>,
) -> ReportResult<()> {
    run.sort_by(sheet_order);
    let rows = u64::try_from(run.len()).map_err(|_| ReportError::Invariant {
        detail: "a sorted run does not fit its row count".to_string(),
    })?;
    let file = run_file(0, *index);
    let path = dir.join(&file);
    *index = index.checked_add(1).ok_or_else(|| ReportError::Invariant {
        detail: "the spill wrote more runs than it can count".to_string(),
    })?;
    write_run(&path, run)?;
    run.clear();
    let meta = RunMeta { file, rows, bytes };
    tracing::debug!(
        file = meta.file.as_str(),
        rows = meta.rows,
        bytes = meta.bytes,
        "spilled a sorted run"
    );
    spilled.try_reserve(1).map_err(|_| ReportError::Invariant {
        detail: "the performance spill cannot record its sorted run".to_string(),
    })?;
    spilled.push(meta);
    Ok(())
}

fn write_run(path: &Path, run: &[PerformanceRow]) -> ReportResult<()> {
    let file = File::create(path).map_err(|source| io_error(path, source))?;
    let mut writer = BufWriter::new(file);
    for row in run {
        serde_json::to_writer(&mut writer, row).map_err(|source| io_error(path, source.into()))?;
        writer
            .write_all(b"\n")
            .map_err(|source| io_error(path, source))?;
    }
    writer.flush().map_err(|source| io_error(path, source))?;
    Ok(())
}

fn collapse_runs(dir: &Path, runs: Vec<RunMeta>, max_ranges: usize) -> ReportResult<Vec<RunMeta>> {
    let mut runs = runs;
    let mut passes: usize = 0;
    while runs.len() > max_ranges {
        passes = passes
            .checked_add(1)
            .ok_or_else(|| ReportError::Invariant {
                detail: "the spill cannot count its collapse passes".to_string(),
            })?;
        if passes > MAX_COLLAPSE_PASSES {
            return Err(ReportError::Invariant {
                detail: "the spill cannot collapse its runs within its pass budget".to_string(),
            });
        }
        runs = collapse_pass(dir, runs, max_ranges.max(COLLAPSE_FANIN_FLOOR), passes)?;
    }
    Ok(runs)
}

fn collapse_pass(
    dir: &Path,
    runs: Vec<RunMeta>,
    fanin: usize,
    pass: usize,
) -> ReportResult<Vec<RunMeta>> {
    let mut merged = Vec::new();
    merged
        .try_reserve(runs.len().div_ceil(fanin.max(1)))
        .map_err(|_| ReportError::Invariant {
            detail: "the performance spill cannot record its collapsed runs".to_string(),
        })?;
    for (index, group) in runs.chunks(fanin.max(1)).enumerate() {
        let file = run_file(pass, index);
        let path = dir.join(&file);
        let (rows, bytes) = merge_group(dir, group, &path)?;
        for meta in group {
            if let Err(error) = std::fs::remove_file(dir.join(&meta.file)) {
                tracing::warn!(
                    path = %dir.join(&meta.file).display(),
                    %error,
                    "could not remove a collapsed spill run"
                );
            }
        }
        let meta = RunMeta { file, rows, bytes };
        tracing::debug!(
            file = meta.file.as_str(),
            rows = meta.rows,
            bytes = meta.bytes,
            "collapsed spill runs"
        );
        merged.push(meta);
    }
    Ok(merged)
}

fn merge_group(dir: &Path, group: &[RunMeta], path: &Path) -> ReportResult<(u64, u64)> {
    let mut merge = RunMerge::open(None, run_paths(dir, group)?)?;
    let file = File::create(path).map_err(|source| io_error(path, source))?;
    let mut writer = BufWriter::new(file);
    let mut rows: u64 = 0;
    let mut bytes: u64 = 0;
    while let Some(row) = merge.next_row()? {
        let mut encoded = serde_json::to_vec(&row).map_err(|_| ReportError::Invariant {
            detail: "a performance row does not serialize".to_string(),
        })?;
        encoded.push(b'\n');
        writer
            .write_all(&encoded)
            .map_err(|source| io_error(path, source))?;
        rows = rows.checked_add(1).ok_or_else(|| ReportError::Invariant {
            detail: "a collapsed run does not fit its row count".to_string(),
        })?;
        bytes = bytes
            .checked_add(
                u64::try_from(encoded.len()).map_err(|_| ReportError::Invariant {
                    detail: "a collapsed run does not fit its byte count".to_string(),
                })?,
            )
            .ok_or_else(|| ReportError::Invariant {
                detail: "a collapsed run does not fit its byte count".to_string(),
            })?;
    }
    writer.flush().map_err(|source| io_error(path, source))?;
    Ok((rows, bytes))
}

struct RunMerge {
    dir: Option<SpillDir>,
    readers: Vec<RunReader>,
    heap: BinaryHeap<MergeItem>,
    live_rows: usize,
    live_bytes: u64,
    max_live_rows: usize,
    max_live_bytes: u64,
}

struct RunReader {
    file: String,
    path: PathBuf,
    reader: BufReader<File>,
    line: String,
    line_number: usize,
    exhausted: bool,
}

struct MergeItem {
    row: PerformanceRow,
    bytes: u64,
    run: usize,
}

impl PartialEq for MergeItem {
    fn eq(&self, other: &Self) -> bool {
        self.row == other.row && self.run == other.run
    }
}

impl Eq for MergeItem {}

impl PartialOrd for MergeItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MergeItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        sheet_order(&other.row, &self.row).then_with(|| other.run.cmp(&self.run))
    }
}

impl RunMerge {
    fn open(dir: Option<SpillDir>, files: Vec<(PathBuf, String)>) -> ReportResult<Self> {
        let mut readers = Vec::new();
        readers
            .try_reserve(files.len())
            .map_err(|_| ReportError::Invariant {
                detail: "the performance spill cannot open its sorted runs".to_string(),
            })?;
        for (path, file) in files {
            readers.push(RunReader::open(path, file)?);
        }
        let mut heap = BinaryHeap::new();
        heap.try_reserve(readers.len())
            .map_err(|_| ReportError::Invariant {
                detail: "the performance spill cannot stage its sorted runs".to_string(),
            })?;
        let mut merge = Self {
            dir,
            readers,
            heap,
            live_rows: 0,
            live_bytes: 0,
            max_live_rows: 0,
            max_live_bytes: 0,
        };
        let live = merge.readers.len();
        for run in 0..live {
            merge.refill(run)?;
        }
        Ok(merge)
    }

    fn next_row(&mut self) -> ReportResult<Option<PerformanceRow>> {
        let Some(item) = self.heap.pop() else {
            return Ok(None);
        };
        self.live_rows = self
            .live_rows
            .checked_sub(1)
            .ok_or_else(|| ReportError::Invariant {
                detail: "the performance spill lost count of its live rows".to_string(),
            })?;
        self.live_bytes =
            self.live_bytes
                .checked_sub(item.bytes)
                .ok_or_else(|| ReportError::Invariant {
                    detail: "the performance spill lost count of its live bytes".to_string(),
                })?;
        self.refill(item.run)?;
        Ok(Some(item.row))
    }

    fn refill(&mut self, run: usize) -> ReportResult<()> {
        if let Some((row, row_bytes)) = self.read_row(run)? {
            self.heap
                .try_reserve(1)
                .map_err(|_| ReportError::Invariant {
                    detail: "the performance spill cannot stage its next row".to_string(),
                })?;
            self.heap.push(MergeItem {
                row,
                bytes: row_bytes,
                run,
            });
            self.live_rows =
                self.live_rows
                    .checked_add(1)
                    .ok_or_else(|| ReportError::Invariant {
                        detail: "the performance spill cannot count its live rows".to_string(),
                    })?;
            self.live_bytes =
                self.live_bytes
                    .checked_add(row_bytes)
                    .ok_or_else(|| ReportError::Invariant {
                        detail: "the performance spill cannot count its live bytes".to_string(),
                    })?;
            self.max_live_rows = self.max_live_rows.max(self.live_rows);
            self.max_live_bytes = self.max_live_bytes.max(self.live_bytes);
        }
        Ok(())
    }

    fn read_row(&mut self, run: usize) -> ReportResult<Option<(PerformanceRow, u64)>> {
        let reader = self
            .readers
            .get_mut(run)
            .ok_or_else(|| ReportError::Invariant {
                detail: "the performance spill answers a run it never opened".to_string(),
            })?;
        loop {
            if reader.exhausted {
                return Ok(None);
            }
            reader.line.clear();
            let bytes = reader
                .reader
                .read_line(&mut reader.line)
                .map_err(|source| io_error(&reader.path, source))?;
            if bytes == 0 {
                reader.exhausted = true;
                if let Some(dir) = self.dir.as_ref() {
                    if let Err(error) = std::fs::remove_file(dir.path().join(&reader.file)) {
                        tracing::warn!(
                            path = %dir.path().join(&reader.file).display(),
                            %error,
                            "could not remove an exhausted spill run"
                        );
                    }
                }
                return Ok(None);
            }
            reader.line_number =
                reader
                    .line_number
                    .checked_add(1)
                    .ok_or_else(|| ReportError::Invariant {
                        detail: "a spill run does not fit its line count".to_string(),
                    })?;
            if reader.line.as_str().trim().is_empty() {
                continue;
            }
            let line_number = reader.line_number;
            let row: PerformanceRow =
                serde_json::from_str(reader.line.as_str()).map_err(|source| {
                    ReportError::Decode {
                        path: reader.path.clone(),
                        line: line_number,
                        source,
                    }
                })?;
            let row_bytes = u64::try_from(bytes).map_err(|_| ReportError::Invariant {
                detail: "a spill line does not fit its byte count".to_string(),
            })?;
            reader.line.clear();
            return Ok(Some((row, row_bytes)));
        }
    }
}

impl RunReader {
    fn open(path: PathBuf, file: String) -> ReportResult<Self> {
        let reader = File::open(&path)
            .map(BufReader::new)
            .map_err(|source| io_error(&path, source))?;
        Ok(Self {
            file,
            path,
            reader,
            line: String::new(),
            line_number: 0,
            exhausted: false,
        })
    }
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

#[cfg(test)]
mod tests;
