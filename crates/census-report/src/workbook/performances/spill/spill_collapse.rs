use super::spill_merge::RunMerge;
use super::{run_paths, RunMeta};
use crate::report::{io_error, ReportError, ReportResult};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

pub(super) fn collapse_runs(
    dir: &Path,
    runs: Vec<RunMeta>,
    max_ranges: usize,
) -> ReportResult<Vec<RunMeta>> {
    let mut runs = runs;
    let mut passes: usize = 0;
    while runs.len() > max_ranges {
        passes = next_pass(passes)?;
        if passes > super::MAX_COLLAPSE_PASSES {
            return Err(ReportError::Invariant {
                detail: "the spill cannot collapse its runs within its pass budget".to_string(),
            });
        }
        runs = collapse_pass(
            dir,
            runs,
            max_ranges.max(super::COLLAPSE_FANIN_FLOOR),
            passes,
        )?;
    }
    Ok(runs)
}

fn next_pass(passes: usize) -> ReportResult<usize> {
    passes.checked_add(1).ok_or_else(|| ReportError::Invariant {
        detail: "the spill cannot count its collapse passes".to_string(),
    })
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
        merged.push(collapse_group(dir, group, pass, index)?);
    }
    Ok(merged)
}

fn collapse_group(
    dir: &Path,
    group: &[RunMeta],
    pass: usize,
    index: usize,
) -> ReportResult<RunMeta> {
    let file = super::run_file(pass, index);
    let path = dir.join(&file);
    let (rows, bytes) = merge_group(dir, group, &path)?;
    remove_inputs(dir, group);
    let meta = RunMeta { file, rows, bytes };
    tracing::debug!(
        file = meta.file.as_str(),
        rows = meta.rows,
        bytes = meta.bytes,
        "collapsed spill runs"
    );
    Ok(meta)
}

fn remove_inputs(dir: &Path, group: &[RunMeta]) {
    for meta in group {
        if let Err(error) = std::fs::remove_file(dir.join(&meta.file)) {
            tracing::warn!(
                path = %dir.join(&meta.file).display(),
                %error,
                "could not remove a collapsed spill run"
            );
        }
    }
}

fn merge_group(dir: &Path, group: &[RunMeta], path: &Path) -> ReportResult<(u64, u64)> {
    let mut merge = RunMerge::open(None, run_paths(dir, group)?)?;
    let file = File::create(path).map_err(|source| io_error(path, source))?;
    let mut writer = BufWriter::new(file);
    let mut rows: u64 = 0;
    let mut bytes: u64 = 0;
    while let Some(row) = merge.next_row()? {
        let encoded = append_row(&mut writer, path, &row)?;
        rows = rows.checked_add(1).ok_or_else(|| ReportError::Invariant {
            detail: "a collapsed run does not fit its row count".to_string(),
        })?;
        bytes = bytes
            .checked_add(encoded)
            .ok_or_else(|| ReportError::Invariant {
                detail: "a collapsed run does not fit its byte count".to_string(),
            })?;
    }
    writer.flush().map_err(|source| io_error(path, source))?;
    Ok((rows, bytes))
}

fn append_row(
    writer: &mut BufWriter<File>,
    path: &Path,
    row: &super::PerformanceRow,
) -> ReportResult<u64> {
    let mut encoded = serde_json::to_vec(&row).map_err(|_| ReportError::Invariant {
        detail: "a performance row does not serialize".to_string(),
    })?;
    encoded.push(b'\n');
    writer
        .write_all(&encoded)
        .map_err(|source| io_error(path, source))?;
    u64::try_from(encoded.len()).map_err(|_| ReportError::Invariant {
        detail: "a collapsed run does not fit its byte count".to_string(),
    })
}
