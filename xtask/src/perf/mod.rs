//! Performance baseline verbs: `record`, `check`, and `profile`.
//!
//! These three verbs measure the `census-service` criterion bench targets (`core` and `pipeline`)
//! and maintain a committed throughput baseline. They differ from the existing [`baseline`] module
//! (`quality-baseline`, `ratchet`) which compares debt counts (clippy tallies, scan metrics) and
//! never measures timings or throughput. The perf baseline captures wall time, throughput and peak
//! RSS per group, stamped with hardware and toolchain metadata so that numbers are only comparable
//! against a baseline recorded on the same machine with the same toolchain.
//!
//! ## Baseline schema (written by `perf record`)
//!
//! The JSON file at `tools/perf-baseline.json` carries:
//!
//! - `metadata` — environment stamp:
//!   - `cpu`: CPU model name (`/proc/cpuinfo`, `"unknown"` if absent).
//!   - `cores`: physical core count (`nproc --physical`, 0 on failure).
//!   - `rustc`: rustc version string.
//!   - `sha`: git commit SHA of the working tree.
//!   - `corpus_lines`: total lines of fixture text files, 0 if absent.
//! - `check_reason` — the `--reason` flag from `perf check`, if provided (for audit trail).
//! - `groups` — keyed by criterion group name (e.g. `census/parse`), each carrying:
//!   - `throughput`: elements/s, `null` if the benchmark target does not declare
//!     `Throughput` (criterion only reports it when `.throughput()` is called in the bench fn).
//!   - `peak_rss_kib`: peak resident set size of the Criterion process tree in KiB,
//!     `"not measured"` if `/usr/bin/time` is absent (never the shell's own RSS).
//!   - `wall_time_seconds`: wall time in seconds as reported by criterion.
//!
//! ## `perf record`
//!
//! Runs `cargo bench -p census-service --bench core` and `--bench pipeline`, captures peak RSS
//! via `/usr/bin/time -v` (parse `Maximum resident set size`), and writes the baseline JSON.
//!
//! ## `perf check`
//!
//! Re-runs the same two bench targets, validates that the measuring environment is compatible
//! (CPU model, physical core count, rustc version must match), then compares per-group throughput
//! against the recorded baseline. A group that regresses past `--tolerance` (default 0.05, i.e.
//! 5%) fails the verb and exits non-zero. The tolerance is overridable with `--tolerance <f64>`.
//! A `--reason` flag annotates the comparison so the reason for running the check is recorded
//! alongside the verdict in the baseline.
//!
//! ## `perf profile`
//!
//! Runs one named group under `perf record --call-graph=dwarf` when `perf` is available on the
//! system. If `perf` is absent, prints the exact command it would run and explains why it could
//! not execute. The group name is a criterion group id such as `census/parse`.
//!
//! ## Peak RSS measurement
//!
//! Peak RSS is measured from the Criterion process tree (not the shell) via `/usr/bin/time -v`,
//! which reports `Maximum resident set size`. If `/usr/bin/time` is absent or its output cannot
//! be parsed, the field is recorded as `"not measured"` with the reason. No new dependencies are
//! added; this relies on GNU `time` which is standard on Linux workstations.

mod baseline;
mod bench;
mod compare;
mod env;

pub use baseline::load_baseline;
pub use bench::run_benchmarks;

use crate::cmd::Cmd;
use crate::paths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;

/// Default tolerance for the check verb: 5% throughput regression is a failure.
#[allow(dead_code)]
pub(crate) const DEFAULT_TOLERANCE: f64 = 0.05;

/// Baseline file name.
const BASELINE_FILE: &str = "perf-baseline.json";

/// A single group's measurement as recorded by `perf record`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct GroupMeasurement {
    /// Throughput in elements per second. `null` when the criterion target does not declare
    /// `Throughput` (criterion can only report it if the bench function calls `.throughput()`).
    #[serde(skip_serializing_if = "Option::is_none")]
    throughput: Option<f64>,
    /// Peak resident set size in KiB of the Criterion process tree (measured via `/usr/bin/time -v`
    /// `Maximum resident set size`). `null` when the harness cannot measure it (e.g. GNU time is
    /// not installed on the workstation).
    #[serde(skip_serializing_if = "Option::is_none")]
    peak_rss_kib: Option<u64>,
    /// Wall time in seconds as reported by criterion.
    wall_time_seconds: f64,
}

/// The full perf baseline: metadata about the measuring environment plus per-group measurements.
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct PerfBaseline {
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
pub(crate) struct Meta {
    /// CPU model name from `/proc/cpuinfo`.
    cpu: String,
    /// Physical core count from `nproc --physical`.
    cores: u32,
    /// Rust compiler version string.
    rustc: String,
    /// Git commit SHA of the repository at record time (reported, not required to match for check).
    sha: String,
    /// Total lines of fixture text files, if the fixture directory exists.
    corpus_lines: u64,
}

/// Run `perf record`: run the two criterion bench targets, capture per-group measurements,
/// and write the baseline JSON to `tools/perf-baseline.json`.
///
/// The wall time, throughput and peak RSS are captured from the criterion output and `/usr/bin/time
/// -v`. If a number cannot be measured (e.g. throughput is not declared in the benchmark, or GNU
/// time is absent), the field is written as `null` with no guess or estimate.
pub fn run_record() -> Result<()> {
    let benchmark_data = run_benchmarks()?;
    let meta = env::build_meta()?;
    let baseline = PerfBaseline {
        metadata: meta,
        check_reason: None,
        groups: benchmark_data,
    };
    let baseline_path = baseline::baseline_path();
    let json = serde_json::to_string_pretty(&baseline).with_context(|| "serializing baseline")?;
    fs::write(&baseline_path, json)
        .with_context(|| format!("writing baseline to {}", paths::relative(&baseline_path)))?;
    println!("wrote perf baseline to {}", paths::relative(&baseline_path));
    Ok(())
}

/// Run `perf check`: re-run the bench targets, compare against the recorded baseline, and fail
/// when throughput regresses past the tolerance.
///
/// Before comparing throughput, validates that the measuring environment is compatible with the
/// baseline: CPU model, physical core count and rustc version must all match. The git SHA is
/// reported but not required to match (benchmarks should be comparable across commits on the same
/// machine). A mismatch in any of the three environment fields prints a warning but still
/// proceeds with the throughput comparison.
///
/// The tolerance defaults to 5% and can be overridden with `--tolerance <f64>`. A `--reason` flag
/// annotates the comparison.
pub fn run_check(tolerance: f64, reason: Option<String>) -> Result<()> {
    let baseline = load_baseline()?;
    let current_data = run_benchmarks()?;

    compare::check_environment(&baseline, &current_data)?;
    if let Some(reason) = &reason {
        println!("check reason: {reason}");
    }

    compare::check_throughput(&baseline, &current_data, tolerance)
}

/// Run `perf profile`: execute one named group under `perf record` when `perf` is available,
/// or print the exact command it would run and explain why it did not.
///
/// The group name is a criterion group id (e.g. `census/parse`, `pipeline/result_file`).
pub fn run_profile(group: &str) -> Result<()> {
    if !env::perf_available() {
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

#[cfg(test)]
#[path = "perf_tests.rs"]
mod tests;
