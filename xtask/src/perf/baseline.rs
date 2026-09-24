//! Baseline file I/O.

use super::{PerfBaseline, BASELINE_FILE};
use crate::paths;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Read the perf baseline from the tools directory.
pub fn load_baseline() -> Result<PerfBaseline> {
    let path = baseline_path();
    let content = fs::read_to_string(&path)
        .with_context(|| format!("reading perf baseline: {}", paths::relative(&path)))?;
    let baseline: PerfBaseline =
        serde_json::from_str(&content).with_context(|| "parsing perf baseline JSON")?;
    Ok(baseline)
}

/// Path to the perf baseline file: `tools/perf-baseline.json` relative to the repo root.
pub fn baseline_path() -> PathBuf {
    paths::repo_root().join("tools").join(BASELINE_FILE)
}
