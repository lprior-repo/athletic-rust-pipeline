mod baseline;
mod bench;
pub(crate) mod compare;
mod env;

pub use baseline::load_baseline;
pub use bench::run_benchmarks;

use crate::cmd::Cmd;
use crate::paths;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;

#[allow(dead_code)]
pub(crate) const DEFAULT_TOLERANCE: f64 = 0.05;

const BASELINE_FILE: &str = "perf-baseline.json";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct GroupMeasurement {
    #[serde(skip_serializing_if = "Option::is_none")]
    throughput: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    peak_rss_kib: Option<u64>,
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
    let baseline = load_baseline()?;
    let current_data = run_benchmarks()?;

    compare::check_environment(&baseline, &current_data)?;
    if let Some(reason) = &reason {
        println!("check reason: {reason}");
    }

    compare::check_throughput(&baseline, &current_data, tolerance)
}

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
