mod baseline;
mod bench;
pub(crate) mod compare;
mod corpus;
mod env;
mod scope;

pub use baseline::load_baseline;
pub use bench::run_benchmarks;

use clap::Subcommand;

use crate::cmd::Cmd;
use crate::paths;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;

const BASELINE_FILE: &str = "perf-baseline.json";

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub(crate) enum Throughput<T> {
    Elements(T),
    Bytes(T),
}

impl<T> Throughput<T> {
    fn value(self) -> T {
        match self {
            Self::Elements(value) | Self::Bytes(value) => value,
        }
    }

    fn map<U>(self, map: impl FnOnce(T) -> U) -> Throughput<U> {
        match self {
            Self::Elements(value) => Throughput::Elements(map(value)),
            Self::Bytes(value) => Throughput::Bytes(map(value)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct GroupMeasurement {
    #[serde(skip_serializing_if = "Option::is_none")]
    throughput: Option<Throughput<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    peak_rss_kib: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allocation_count: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    allocated_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tail_time_seconds: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    timing_scope: Option<String>,
    wall_time_seconds: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct PerfBaseline {
    metadata: Meta,
    #[serde(skip_serializing_if = "Option::is_none")]
    check_reason: Option<String>,
    groups: BTreeMap<String, GroupMeasurement>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Meta {
    cpu: String,
    cores: u32,
    rustc: String,
    sha: String,
    corpus_lines: u64,
    corpus_sha256: String,
}

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

pub fn run_check(tolerance: f64, reason: Option<String>) -> Result<()> {
    compare::validate_tolerance(tolerance)?;
    let baseline = load_baseline()?;
    compare::check_environment(&baseline)?;
    compare::validate_baseline(&baseline)?;
    let current_data = run_benchmarks()?;
    if let Some(reason) = &reason {
        println!("check reason: {reason}");
    }

    compare::check_throughput(&baseline, &current_data, tolerance)
}

pub fn run_profile(group: &str) -> Result<()> {
    if !env::perf_available() {
        println!("perf is not installed on this system");
        println!("would run: perf record --call-graph=dwarf cargo bench -p census-service --bench core -- {} --noplot", group);
        println!("and:       perf record --call-graph=dwarf cargo bench -p census-service --bench pipeline -- {} --noplot", group);
        println!("install perf with: apt install linux-tools-generic  (Debian/Ubuntu)");
        println!("or:              dnf install perf  (Fedora/RHEL)");
        println!("then re-run:   cargo xtask perf profile {}", group);
        return Ok(());
    }

    if group.is_empty() {
        bail!("profile: benchmark group argument is empty");
    }

    let mut failures = Vec::new();
    for bench_name in &["core", "pipeline"] {
        match profile_benchmark(bench_name, group) {
            Ok(()) => {}
            Err(e) => failures.push(format!("{bench_name}: {e}")),
        }
    }

    if !failures.is_empty() {
        for f in &failures {
            println!("{f}");
        }
        bail!("profile: some benchmark groups failed to profile");
    }

    Ok(())
}

fn profile_benchmark(bench_name: &str, group: &str) -> Result<()> {
    println!("profiling {bench_name}: {group}");

    let exit = Cmd::new("perf")
        .args(["record", "--call-graph=dwarf"])
        .args([
            "cargo",
            "bench",
            "-p",
            "census-service",
            "--bench",
            bench_name,
            "--",
            group,
            "--noplot",
        ])
        .run();

    match exit {
        Ok(()) => Ok(()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
#[path = "perf_tests.rs"]
mod tests;

#[derive(Subcommand, Debug)]
pub(crate) enum PerfCommand {
    #[command(
        about = "Record Criterion timing, sample-tail timing, throughput and required GNU time RSS/Valgrind DHAT heap allocation measurements"
    )]
    Record,
    #[command(
        about = "Re-run all three benchmark targets, reject ID or measurement mismatches, and fail regressions beyond the tolerance (default 5%)"
    )]
    Check {
        #[arg(
            help = "Override the default 5% regression tolerance (e.g. `--tolerance 0.1` for 10%)"
        )]
        #[arg(long, default_value_t = 0.05)]
        tolerance: f64,
        #[arg(
            help = "Reason for running the check; does not waive environment or metric mismatches"
        )]
        #[arg(long)]
        reason: Option<String>,
    },
    #[command(
        about = "Run one named group under `perf record --call-graph=dwarf`; prints the exact command and explains why it did not run when `perf` is absent"
    )]
    Profile {
        #[arg(
            help = "Criterion group id to profile, e.g. `census/parse` or `pipeline/result_file`"
        )]
        group: String,
    },
}
