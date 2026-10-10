use super::{run_file, sheet_order, PerformanceRow, RunMeta, RUN_BYTES};
use crate::report::{io_error, ReportError, ReportResult};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

pub(super) fn spill_sorted_runs(
    dir: &Path,
    rows: impl Iterator<Item = PerformanceRow>,
    row_budget: u64,
) -> ReportResult<Vec<RunMeta>> {
    let mut stager = RunStager {
        dir,
        run: Vec::new(),
        run_bytes: 0,
        index: 0,
        spilled: Vec::new(),
        row_budget,
    };
    for row in rows {
        stager.stage(row)?;
    }
    stager.finish()
}

struct RunStager<'a> {
    dir: &'a Path,
    run: Vec<PerformanceRow>,
    run_bytes: u64,
    index: usize,
    spilled: Vec<RunMeta>,
    row_budget: u64,
}

impl RunStager<'_> {
    fn stage(&mut self, row: PerformanceRow) -> ReportResult<()> {
        let row_bytes = sized_row(&row)?;
        self.flush_if_full(row_bytes)?;
        self.run
            .try_reserve(1)
            .map_err(|_| ReportError::Invariant {
                detail: "the performance spill cannot grow its sorted run".to_string(),
            })?;
        self.run.push(row);
        self.run_bytes =
            self.run_bytes
                .checked_add(row_bytes)
                .ok_or_else(|| ReportError::Invariant {
                    detail: "the performance spill cannot count its run bytes".to_string(),
                })?;
        Ok(())
    }

    fn flush_if_full(&mut self, row_bytes: u64) -> ReportResult<()> {
        if run_is_full(&self.run, self.run_bytes, self.row_budget, row_bytes)? {
            let bytes = self.run_bytes;
            flush_run(
                self.dir,
                &mut self.run,
                bytes,
                &mut self.index,
                &mut self.spilled,
            )?;
            self.run_bytes = 0;
        }
        Ok(())
    }

    fn finish(mut self) -> ReportResult<Vec<RunMeta>> {
        if !self.run.is_empty() {
            let bytes = self.run_bytes;
            flush_run(
                self.dir,
                &mut self.run,
                bytes,
                &mut self.index,
                &mut self.spilled,
            )?;
        }
        Ok(self.spilled)
    }
}

fn sized_row(row: &PerformanceRow) -> ReportResult<u64> {
    let row_bytes = encoded_row_bytes(row)?;
    if row_bytes > super::MAX_ROW_BYTES {
        return Err(ReportError::Invariant {
            detail: format!("performance {} exceeds the spill row budget", row.id),
        });
    }
    Ok(row_bytes)
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
    record_run(spilled, file, rows, bytes)
}

fn record_run(spilled: &mut Vec<RunMeta>, file: String, rows: u64, bytes: u64) -> ReportResult<()> {
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
