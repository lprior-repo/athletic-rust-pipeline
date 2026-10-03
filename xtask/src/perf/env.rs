use super::Meta;
use crate::paths;
use anyhow::{Context, Result};
use std::path::Path;

pub fn build_meta() -> Result<Meta> {
    Ok(Meta {
        cpu: cpu_model()?,
        cores: physical_cores()?,
        rustc: rustc_version(),
        sha: git_sha(),
        corpus_lines: corpus_size(),
    })
}

pub fn cpu_model() -> Result<String> {
    let content = std::fs::read_to_string("/proc/cpuinfo")
        .map_or(String::new(), core::convert::identity);
    for line in content.lines() {
        if let Some(model) = line.strip_prefix("model name\t: ") {
            return Ok(model.to_string());
        }
    }
    Ok("unknown".to_string())
}

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

pub fn git_sha() -> String {
    match super::Cmd::new("bash")
        .arg("-c")
        .arg("git rev-parse HEAD 2>/dev/null || echo unknown")
        .output()
    {
        Ok(output) => output.trim().to_string(),
        Err(_) => "unknown".to_string(),
    }
}

pub fn rustc_version() -> String {
    match super::Cmd::new("bash")
        .arg("-c")
        .arg("rustc --version 2>/dev/null || echo unknown")
        .output()
    {
        Ok(output) => output.trim().to_string(),
        Err(_) => "unknown".to_string(),
    }
}

pub fn corpus_size() -> u64 {
    fn count_lines_in_dir(dir: &Path) -> u64 {
        let mut total = 0u64;
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "txt" || ext == "htm" || ext == "html" {
                            let lines = std::fs::read_to_string(&path)
                                .ok()
                                .map(|s| {
                                    u64::try_from(s.lines().count())
                                        .map_or(u64::MAX, core::convert::identity)
                                })
                                .map_or(0, core::convert::identity);
                            total = total.saturating_add(lines);
                        }
                    }
                } else if path.is_dir() {
                    total = total.saturating_add(count_lines_in_dir(&path));
                }
            }
        }
        total
    }
    count_lines_in_dir(&paths::fixtures_dir())
}

pub fn perf_available() -> bool {
    std::process::Command::new("perf")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}
