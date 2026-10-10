use super::spill_dir::SpillDir;
use super::{PerformanceRow, MAX_MERGE_BYTES, MAX_RANGES};
use crate::report::{io_error, ReportError, ReportResult};
use std::collections::BinaryHeap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

pub(super) struct RunMerge {
    pub(super) dir: Option<SpillDir>,
    pub(super) readers: Vec<RunReader>,
    pub(super) heap: BinaryHeap<MergeItem>,
    pub(super) live_rows: usize,
    pub(super) live_bytes: u64,
    pub(super) max_live_rows: usize,
    pub(super) max_live_bytes: u64,
}

pub(super) struct RunReader {
    file: String,
    path: PathBuf,
    reader: BufReader<File>,
    line: String,
    line_number: usize,
    exhausted: bool,
}

pub(super) struct MergeItem {
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
        super::sheet_order(&other.row, &self.row).then_with(|| other.run.cmp(&self.run))
    }
}

enum LineRead {
    Exhausted,
    Blank,
    Row(Box<PerformanceRow>, u64),
}

impl RunMerge {
    pub(super) fn open(dir: Option<SpillDir>, files: Vec<(PathBuf, String)>) -> ReportResult<Self> {
        check_merge_width(&files)?;
        let readers = open_readers(&files)?;
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

    pub(super) fn next_row(&mut self) -> ReportResult<Option<PerformanceRow>> {
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
            self.admit(row, row_bytes, run)?;
        }
        Ok(())
    }

    fn admit(&mut self, row: PerformanceRow, row_bytes: u64, run: usize) -> ReportResult<()> {
        let live_bytes =
            self.live_bytes
                .checked_add(row_bytes)
                .ok_or_else(|| ReportError::Invariant {
                    detail: "the performance spill cannot count its live bytes".to_string(),
                })?;
        if live_bytes > MAX_MERGE_BYTES {
            return Err(ReportError::Invariant {
                detail: "the performance spill refuses a merge beyond its live byte budget"
                    .to_string(),
            });
        }
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
        self.account_live(live_bytes)
    }

    fn account_live(&mut self, live_bytes: u64) -> ReportResult<()> {
        self.live_rows = self
            .live_rows
            .checked_add(1)
            .ok_or_else(|| ReportError::Invariant {
                detail: "the performance spill cannot count its live rows".to_string(),
            })?;
        self.live_bytes = live_bytes;
        self.max_live_rows = self.max_live_rows.max(self.live_rows);
        self.max_live_bytes = self.max_live_bytes.max(self.live_bytes);
        Ok(())
    }

    fn read_row(&mut self, run: usize) -> ReportResult<Option<(PerformanceRow, u64)>> {
        let Some(entry) = self.readers.get_mut(run) else {
            return Err(ReportError::Invariant {
                detail: "the performance spill answers a run it never opened".to_string(),
            });
        };
        loop {
            match next_line(entry)? {
                LineRead::Exhausted => {
                    Self::remove_run_file(self.dir.as_ref(), &entry.file);
                    entry.exhausted = true;
                    return Ok(None);
                }
                LineRead::Blank => {}
                LineRead::Row(row, row_bytes) => return Ok(Some((*row, row_bytes))),
            }
        }
    }

    fn remove_run_file(dir: Option<&SpillDir>, file: &str) {
        if let Some(dir) = dir {
            if let Err(error) = std::fs::remove_file(dir.path().join(file)) {
                tracing::warn!(
                    path = %dir.path().join(file).display(),
                    %error,
                    "could not remove an exhausted spill run"
                );
            }
        }
    }
}

fn next_line(reader: &mut RunReader) -> ReportResult<LineRead> {
    if reader.exhausted {
        return Ok(LineRead::Exhausted);
    }
    reader.line.clear();
    let bytes = reader
        .reader
        .read_line(&mut reader.line)
        .map_err(|source| io_error(&reader.path, source))?;
    if bytes == 0 {
        return Ok(LineRead::Exhausted);
    }
    reader.line_number =
        reader
            .line_number
            .checked_add(1)
            .ok_or_else(|| ReportError::Invariant {
                detail: "a spill run does not fit its line count".to_string(),
            })?;
    decode_line(reader, bytes)
}

fn decode_line(reader: &mut RunReader, bytes: usize) -> ReportResult<LineRead> {
    if reader.line.as_str().trim().is_empty() {
        return Ok(LineRead::Blank);
    }
    let line_number = reader.line_number;
    let row: PerformanceRow =
        serde_json::from_str(reader.line.as_str()).map_err(|source| ReportError::Decode {
            path: reader.path.clone(),
            line: line_number,
            source,
        })?;
    let row_bytes = u64::try_from(bytes).map_err(|_| ReportError::Invariant {
        detail: "a spill line does not fit its byte count".to_string(),
    })?;
    reader.line.clear();
    Ok(LineRead::Row(Box::new(row), row_bytes))
}

fn check_merge_width(files: &[(PathBuf, String)]) -> ReportResult<()> {
    if files.len() > MAX_RANGES {
        return Err(ReportError::Invariant {
            detail: "the performance spill refuses a merge wider than its run budget".to_string(),
        });
    }
    Ok(())
}

fn open_readers(files: &[(PathBuf, String)]) -> ReportResult<Vec<RunReader>> {
    let mut readers = Vec::new();
    readers
        .try_reserve(files.len())
        .map_err(|_| ReportError::Invariant {
            detail: "the performance spill cannot open its sorted runs".to_string(),
        })?;
    for (path, file) in files {
        readers.push(RunReader::open(path.clone(), file.clone())?);
    }
    Ok(readers)
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
