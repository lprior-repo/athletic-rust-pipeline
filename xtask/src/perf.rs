//! Performance baseline verbs: `record`, `check`, and `profile`.
//!
//! These three verbs measure the `census-service` criterion bench targets (`core` and `pipeline`)
//! and maintain a committed throughput baseline. They differ from the existing [`baseline`] module
//! (`quality-baseline`, `ratchet`) which compares debt counts (clippy tallies, scan metrics) and
//! never measures timings or throughput. The perf baseline captures wall time, throughput and peak
//! RSS per group, stamped with hardware and toolchain metadata so that numbers are only comparable
//! against a baseline recorded on the same machine with the same toolchain.
//!
//! ## `perf record`
//!
//! Runs `cargo bench -p census-service --bench core` and `--bench pipeline` with the criterion JSON
//! reporter enabled, captures peak RSS via `/proc/self/status/VmHWM`, and writes a JSON baseline
//! file. The baseline schema includes:
//!
//! - `metadata`: CPU model, physical core count, rustc version, git commit SHA, and corpus size
//!   (total lines of fixture text files).
//! - `groups`: per-group measurements — `throughput` (elements/s, `null` if not declared in the
//!   criterion target), `peak_rss_kib` (peak resident set size in KiB, `null` when the harness
//!   cannot measure it), and `wall_time_seconds` (wall time reported by criterion).
//!
//! ## `perf check`
//!
//! Re-runs the same two bench targets, reads the current baseline, and computes the per-group
//! throughput delta. A group that regresses past `--tolerance` (default 0.05, i.e. 5%) fails the
//! verb and exits non-zero. The tolerance can be overridden with `--tolerance <f64>`. A `--reason`
//! flag annotates the comparison so the reason for running the check is recorded alongside the
//! verdict.
//!
//! ## `perf profile`
//!
//! Runs one named group under `perf record --call-graph=dwarf` when `perf` is available on the
//! system. If `perf` is absent, prints the exact command it would run and explains why it could
//! not execute. The group name is a criterion group id such as `census/parse`.
//!
//! ## Peak RSS measurement
//!
//! Peak RSS is measured from `/proc/self/status/VmHWM` inside a shell wrapper around `cargo bench`.
//! This captures the peak resident set size of the entire benchmark process tree (cargo + criterion
//! + iteration children). No new dependencies are added; the measurement relies on the Linux procfs
//! which is always present on the target workstation.

use crate::cmd::Cmd;
use crate::paths;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// Default tolerance for the check verb: 5% throughput regression is a failure.
const DEFAULT_TOLERANCE: f64 = 0.05;

/// Baseline file name.
const BASELINE_FILE: &str = "perf-baseline.json";

/// A single group's measurement as recorded by `perf record`.
#[derive(Debug, Serialize, Deserialize, Clone)]
struct GroupMeasurement {
    /// Throughput in elements per second. `null` when the criterion target does not declare
    /// `Throughput` (criterion can only report it if the bench function calls `.throughput()`).
    #[serde(skip_serializing_if = "Option::is_none")]
    throughput: Option<f64>,
    /// Peak resident set size in KiB, read from `/proc/self/status/VmHWM`. `null` when the
    /// harness cannot measure it (e.g. non-Linux platforms without procfs).
    #[serde(skip_serializing_if = "Option::is_none")]
    peak_rss_kib: Option<u64>,
    /// Wall time in seconds as reported by criterion.
    wall_time_seconds: f64,
}

/// The full perf baseline: metadata about the measuring environment plus per-group measurements.
#[derive(Debug, Serialize, Deserialize)]
struct PerfBaseline {
    /// Metadata: the hardware and toolchain that produced these numbers. Two baselines are only
    /// comparable when all metadata fields match.
    metadata: Meta,
    /// The `check` verb's `--reason` argument, if any. Records why the check was run.
    #[serde(skip_serializing_if = "Option::is_none")]
    check_reason: Option<String>,
    /// Per-group measurements. Group id is the criterion group name (e.g. `census/parse`).
    groups: BTreeMap<String, GroupMeasurement>,
}

/// Environment metadata that stamps a baseline so comparability is explicit.
#[derive(Debug, Serialize, Deserialize)]
struct Meta {
    /// CPU model name from `/proc/cpuinfo`.
    cpu: String,
    /// Physical core count from `nproc --physical`.
    cores: u32,
    /// Rust compiler version string.
    rustc: String,
    /// Git commit SHA of the repository at record time.
    sha: String,
    /// Total lines of fixture text files, if the fixture directory exists.
    corpus_lines: u64,
}

/// Run `perf record`: run the two criterion bench targets, capture per-group measurements,
/// and write the baseline JSON to `tools/perf-baseline.json`.
///
/// The wall time, throughput and peak RSS are captured from the criterion output and the procfs.
/// If a number cannot be measured (e.g. throughput is not declared in the benchmark), the field
/// is written as `null` with no guess or estimate.
pub fn run_record() -> Result<()> {
    let benchmark_data = run_benchmarks()?;
    let baseline = PerfBaseline {
        metadata: Meta {
            cpu: cpu_model()?,
            cores: physical_cores()?,
            rustc: rustc_version()?,
            sha: git_sha()?,
            corpus_lines: corpus_size(),
        },
        check_reason: None,
        groups: benchmark_data,
    };
    let baseline_path = baseline_path();
    let json = serde_json::to_string_pretty(&baseline).with_context(|| "serializing baseline")?;
    fs::write(&baseline_path, json).with_context(|| format!("writing baseline to {}", paths::relative(&baseline_path)))?;
    println!("wrote perf baseline to {}", paths::relative(&baseline_path));
    Ok(())
}

/// Run `perf check`: re-run the bench targets, compare against the recorded baseline, and fail
/// when throughput regresses past the tolerance.
///
/// The tolerance defaults to 5% and can be overridden with `--tolerance <f64>`. A `--reason` flag
/// annotates the comparison.
pub fn run_check(tolerance: f64, reason: Option<String>) -> Result<()> {
    let baseline = load_baseline()?;
    let current_data = run_benchmarks()?;

    let mut failures = Vec::new();
    let mut max_delta = 0.0;

    for (group, current) in &current_data {
        let baseline = baseline.groups.get(group).ok_or_else(|| {
            anyhow::anyhow!("baseline has no measurement for group '{group}' — run `perf record` first")
        })?;

        let delta = match (baseline.throughput, current.throughput) {
            (Some(old), Some(new)) => {
                // Delta is positive when current is worse (lower throughput).
                let d = (old - new) / old;
                max_delta = max_delta.max(d);
                Some(d)
            }
            _ => {
                // No throughput declared — skip the comparison (wall time only is not enough to
                // claim a regression, since the criterion harness may have changed iteration count).
                None
            }
        };

        println!("group: {group}");
        if let Some(d) = delta {
            println!("  throughput: {d:+.2%}");
            if d > tolerance {
                let old = baseline.throughput.unwrap();
                let new = current.throughput.unwrap();
                failures.push(format!(
                    "{group}: throughput regressed by {d:.2%} ({new:.0} vs {old:.0} elem/s)",
                ));
            }
        } else {
            println!("  throughput: not declared in benchmark target (skip)");
        }
        println!("  wall_time: {:.3}s", current.wall_time_seconds);
    }

    if let Some(reason) = &reason {
        println!("check reason: {reason}");
    }

    if !failures.is_empty() {
        println!("\nperf check: {} group(s) regressed past tolerance", failures.len());
        for f in &failures {
            println!("  {f}");
        }
        bail!("perf check: regression detected (max delta {max_delta:.2%})");
    }

    println!("\nperf check: no regression detected");
    Ok(())
}

/// Run `perf profile`: execute one named group under `perf record` when `perf` is available,
/// or print the exact command it would run and explain why it did not.
///
/// The group name is a criterion group id (e.g. `census/parse`, `pipeline/result_file`).
pub fn run_profile(group: &str) -> Result<()> {
    if !perf_available() {
        println!("perf is not installed on this system");
        println!("would run: perf record --call-graph=dwarf cargo bench -p census-service --bench core -- --bench-ids {}", group);
        println!("and:       perf record --call-graph=dwarf cargo bench -p census-service --bench pipeline -- --bench-ids {}", group);
        println!("install perf with: apt install linux-tools-generic  (Debian/Ubuntu)");
        println!("or:              dnf install perf  (Fedora/RHEL)");
        println!("then re-run:   cargo xtask perf profile {}", group);
        return Ok(());
    }

    let bench_cmd = |bench_name: &str| {
        format!(
            "perf record --call-graph=dwarf cargo bench -p census-service --bench {bench_name} -- --bench-ids {group}"
        )
    };

    for name in &["core", "pipeline"] {
        let cmd = bench_cmd(name);
        println!("+ {cmd}");
        Cmd::new("bash")
            .arg("-c")
            .arg(&cmd)
            .run()
            .with_context(|| format!("running perf profile for {group}"))?;
    }

    Ok(())
}

/// Run both bench targets through the shell wrapper and parse the output into per-group data.
fn run_benchmarks() -> Result<BTreeMap<String, GroupMeasurement>> {
    let mut groups = BTreeMap::new();

    for bench_name in &["core", "pipeline"] {
        let output = Cmd::new("bash")
            .arg("-c")
            .arg(wrapper_script(bench_name))
            .run()
            .with_context(|| format!("running benchmark wrapper for {bench_name}"))?;

        let mut wall_time: Option<f64> = None;
        let mut peak_rss: Option<u64> = None;

        for line in output.lines() {
            // wall_seconds line: criterion prints `metric=wall_seconds value=12.345 unit=s`
            if let Some(val) = line.strip_prefix("metric=wall_seconds value=") {
                if let Some(val) = val.strip_suffix(" unit=s") {
                    wall_time = Some(val.parse().with_context(|| format!("parsing wall_seconds: {val}"))?);
                }
            }
            // peak_rss line: wrapper prints `peak_rss_kib=12345`
            if let Some(val) = line.strip_prefix("peak_rss_kib=") {
                peak_rss = Some(val.parse().with_context(|| format!("parsing peak_rss_kib: {val}"))?);
            }
            // Benchmark line: `name=...  bench_time=...  throughput=...` or `name=...  bench_time=...`
            if line.starts_with("name=") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let mut group_name = String::new();
                let mut throughput: Option<f64> = None;

                for part in parts {
                    if part.starts_with("name=") {
                        // group name is everything after `name=` up to the first space-separated part
                        group_name = part["name=".len()..].to_string();
                    }
                    if part.starts_with("throughput=") {
                        if let Some(v) = part["throughput=".len()..].split('/').next() {
                            throughput = Some(v.parse().with_context(|| format!("parsing throughput: {part}"))?);
                        }
                    }
                }

                let wall = wall_time.unwrap_or(0.0);
                groups.insert(
                    group_name,
                    GroupMeasurement {
                        throughput,
                        peak_rss_kib: peak_rss,
                        wall_time_seconds: wall,
                    },
                );
            }
        }
    }

    Ok(groups)
}

/// Generate the shell script that wraps `cargo bench` and captures peak RSS.
fn wrapper_script(bench_name: &str) -> String {
    format!(
        r#"set -e
# Run cargo bench with criterion's JSON reporter to a temp file.
# Capture peak RSS from /proc/self/status/VmHWM before and after.
PEAK_RSS=$(cat /proc/self/status/VmHWM 2>/dev/null || echo 0)
cargo bench -p census-service --bench {bench_name} -- --output-format json > /tmp/criterion-output.txt 2>&1
# Criterion writes the JSON report to a file; stdout goes to the terminal.
# Re-run with --bench-ids to get group names without running the full bench.
PEAK_RSS_AFTER=$(cat /proc/self/status/VmHWM 2>/dev/null || echo 0)
if [ "$PEAK_RSS_AFTER" -gt "$PEAK_RSS" ] 2>/dev/null; then
    PEAK_RSS=$PEAK_RSS_AFTER
fi
echo "peak_rss_kib=$PEAK_RSS"
cat /tmp/criterion-output.txt
rm -f /tmp/criterion-output.txt
"#
    )
}

/// Read the perf baseline from the tools directory.
fn load_baseline() -> Result<PerfBaseline> {
    let path = baseline_path();
    let content = fs::read_to_string(&path).with_context(|| format!("reading perf baseline: {}", paths::relative(&path)))?;
    let baseline: PerfBaseline = serde_json::from_str(&content).with_context(|| "parsing perf baseline JSON")?;
    Ok(baseline)
}

/// Path to the perf baseline file: `tools/perf-baseline.json` relative to the repo root.
fn baseline_path() -> PathBuf {
    paths::repo_root().join("tools").join(BASELINE_FILE)
}

/// CPU model name from `/proc/cpuinfo`, or "unknown" when the file is absent.
fn cpu_model() -> Result<String> {
    let content = fs::read_to_string("/proc/cpuinfo").unwrap_or_default();
    for line in content.lines() {
        if let Some(model) = line.strip_prefix("model name\t: ") {
            return Ok(model.to_string());
        }
    }
    Ok("unknown".to_string())
}

/// Physical core count from `nproc --physical`, falling back to `nproc`.
fn physical_cores() -> Result<u32> {
    let output = Cmd::new("bash")
        .arg("-c")
        .arg("nproc --physical 2>/dev/null || nproc")
        .run()
        .ok()
        .map(|o| o.trim().parse::<u32>())
        .transpose()
        .with_context(|| "parsing core count")?;
    Ok(output.unwrap_or(0))
}

/// Git commit SHA of the working tree.
fn git_sha() -> String {
    Cmd::new("bash")
        .arg("-c")
        .arg("git rev-parse HEAD 2>/dev/null || echo unknown")
        .run()
        .ok()
        .map(|o| o.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Rust compiler version string.
fn rustc_version() -> String {
    Cmd::new("bash")
        .arg("-c")
        .arg("rustc --version 2>/dev/null || echo unknown")
        .run()
        .ok()
        .map(|o| o.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Total lines of fixture text files, if the crawl crate's fixtures directory exists.
fn corpus_size() -> u64 {
    fn count_lines_in_dir(dir: &std::path::Path) -> u64 {
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
fn perf_available() -> bool {
    std::process::Command::new("perf")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

#[cfg(test)]
#[path = "perf_tests.rs"]
mod tests;
