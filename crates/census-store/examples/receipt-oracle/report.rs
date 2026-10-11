use crate::args::{Args, Fault};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Verdict {
    pub sequence_marker: u64,
    pub sequence_copy: u64,
    pub rows_marker: usize,
    pub rows_copy: usize,
    pub receipts_marker: usize,
    pub receipts_copy: usize,
    pub rows_new: u64,
    pub receipts_new: usize,
    pub integrity_ok: bool,
    pub copy_dir: PathBuf,
    pub limits: Vec<String>,
    pub orphan_flags: Vec<String>,
    pub marker_sha: String,
}

pub fn write_json(path: &Path, value: &serde_json::Value) -> Result<(), Fault> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|error| Fault::io(error.to_string()))?;
    fs::write(path, bytes)
        .map_err(|error| Fault::io(format!("writing {}: {error}", path.display())))
}

pub fn publish(args: &Args, verdict: &Verdict, log: &mut Vec<String>) -> Result<(), Fault> {
    let orphan_flag_count = verdict.orphan_flags.len();
    write_json(
        &args.evidence.join("receipt-oracle.json"),
        &json!({
            "verdict": "pass",
            "kind": args.kind,
            "operation": args.operation,
            "invocation": args.invocation,
            "invocation_bound": false,
            "store": args.store,
            "copy": verdict.copy_dir,
            "marker_sha256": verdict.marker_sha,
            "sequence_marker": verdict.sequence_marker,
            "sequence_copy": verdict.sequence_copy,
            "rows_marker": verdict.rows_marker,
            "rows_copy": verdict.rows_copy,
            "receipts_marker": verdict.receipts_marker,
            "receipts_copy": verdict.receipts_copy,
            "rows_new": verdict.rows_new,
            "receipts_new": verdict.receipts_new,
            "integrity_ok": verdict.integrity_ok,
            "orphan_flag_count": orphan_flag_count,
            "orphan_flags": verdict.orphan_flags,
            "limits": verdict.limits,
        }),
    )?;
    log.push(format!(
        "INFO: conservation verified (sequence marker={} copy={}; rows marker={} copy={}; receipts marker={} copy={})",
        verdict.sequence_marker,
        verdict.sequence_copy,
        verdict.rows_marker,
        verdict.rows_copy,
        verdict.receipts_marker,
        verdict.receipts_copy
    ));
    Ok(())
}
