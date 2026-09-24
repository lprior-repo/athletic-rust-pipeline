//! Environment detection: CPU, cores, toolchain, git, corpus size.

use super::Meta;
use crate::paths;
use anyhow::{Context, Result};
use std::path::Path;

/// Build environment metadata from the current system.
pub fn build_meta() -> Result<Meta> {
    Ok(Meta {
        cpu: cpu_model()?,
        cores: physical_cores()?,
        rustc: rustc_version(),
        sha: git_sha(),
        corpus_lines: corpus_size(),
    })
}

/// CPU model name from `/proc/cpuinfo`, or "unknown" when the file is absent.
pub fn cpu_model() -> Result<String> {
    let content = std::fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    for line in content.lines() {
        if let Some(model) = line.strip_prefix("model name\t: ") {
            return Ok(model.to_string());
        }
    }
    Ok("unknown".to_string())
}

/// Physical core count from `nproc --physical`, falling back to `nproc`.
pub fn physical_cores() -> Result<u32> {
    let output = super::Cmd::new("bash")
        .arg("-c")
        .arg("nproc --physical 2>/dev/null || nproc")
        .output()?;
    output
        .trim()
        .parse::<u32>()
        .with_context(|| "parsing nproc --physical")
}

/// Git commit SHA of the working tree.
pub fn git_sha() -> String {
    super::Cmd::new("bash")
        .arg("-c")
        .arg("git rev-parse HEAD 2>/dev/null || echo unknown")
        .output()
        .map(|o| o.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Rust compiler version string.
pub fn rustc_version() -> String {
    super::Cmd::new("bash")
        .arg("-c")
        .arg("rustc --version 2>/dev/null || echo unknown")
        .output()
        .map(|o| o.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

/// Total lines of fixture text files, if the crawl crate's fixtures directory exists.
pub fn corpus_size() -> u64 {
    fn count_lines_in_dir(dir: &Path) -> u64 {
        let mut total = 0u64;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "txt" || ext == "htm" || ext == "html" {
                            total += std::fs::read_to_string(&path)
                                .ok()
                                .map(|s| s.lines().count() as u64)
                                .unwrap_or(0);
                        }
                    }
                } else if path.is_dir() {
                    total += count_lines_in_dir(&path);
                }
            }
        }
        total
    }
    count_lines_in_dir(&paths::fixtures_dir())
}

/// Whether `perf` is available on the system.
pub fn perf_available() -> bool {
    std::process::Command::new("perf")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}
