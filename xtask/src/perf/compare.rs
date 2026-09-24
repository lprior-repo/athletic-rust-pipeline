//! Comparison logic: environment compatibility and throughput regression detection.

use super::env;
use super::{GroupMeasurement, PerfBaseline};
use anyhow::{bail, Result};
use std::collections::BTreeMap;

/// Validate environment compatibility between baseline and current measurements.
pub fn check_environment(
    baseline: &PerfBaseline,
    _current_data: &BTreeMap<String, GroupMeasurement>,
) -> Result<()> {
    let current_meta = env::build_meta()?;

    let mut env_warnings = Vec::new();
    if baseline.metadata.cpu != current_meta.cpu {
        env_warnings.push(format!(
            "CPU mismatch: baseline={} current={}",
            baseline.metadata.cpu, current_meta.cpu
        ));
    }
    if baseline.metadata.cores != current_meta.cores {
        env_warnings.push(format!(
            "Core count mismatch: baseline={} current={}",
            baseline.metadata.cores, current_meta.cores
        ));
    }
    if baseline.metadata.rustc != current_meta.rustc {
        env_warnings.push(format!(
            "rustc mismatch: baseline={} current={}",
            baseline.metadata.rustc, current_meta.rustc
        ));
    }

    if !env_warnings.is_empty() {
        println!("\nEnvironment differences detected (throughput comparison may be invalid):");
        for w in &env_warnings {
            println!("  {w}");
        }
        println!(
            "\nbaseline sha: {}  current sha: {}",
            baseline.metadata.sha, current_meta.sha
        );
        println!("sha is reported but not required to match");
    }
    Ok(())
}

/// Compare throughput per group against the baseline; returns error on regression.
pub fn check_throughput(
    baseline: &PerfBaseline,
    current_data: &BTreeMap<String, GroupMeasurement>,
    tolerance: f64,
) -> Result<()> {
    let mut max_delta: f64 = 0.0;
    let mut failures = Vec::new();

    for (group, current) in current_data {
        let delta = check_group(&baseline.groups, group, current, tolerance);
        if let Some(d) = delta.max_delta {
            max_delta = max_delta.max(d);
        }
        if let Some(f) = delta.failure {
            failures.push(f);
        }
    }

    if !failures.is_empty() {
        println!(
            "\nperf check: {} group(s) regressed past tolerance",
            failures.len()
        );
        for f in &failures {
            println!("  {f}");
        }
        bail!(
            "perf check: regression detected (max delta {:.2}%)",
            max_delta * 100.0
        );
    }

    println!("\nperf check: no regression detected");
    Ok(())
}

/// Check a single group's throughput against the baseline.
fn check_group<'a>(
    groups: &'a BTreeMap<String, GroupMeasurement>,
    group: &str,
    current: &'a GroupMeasurement,
    tolerance: f64,
) -> GroupCheckResult {
    let baseline = groups.get(group).unwrap_or_else(|| {
        panic!("baseline has no measurement for group '{group}' — run `perf record` first")
    });

    let delta = match (baseline.throughput, current.throughput) {
        (Some(old), Some(new)) => {
            let d = (old - new) / old;
            Some((d, old, new))
        }
        _ => None,
    };

    println!("group: {group}");
    let mut failure: Option<String> = None;
    if let Some((d, old, new)) = delta {
        println!("  throughput: {:.2}%", d * 100.0);
        if d > tolerance {
            failure = Some(format!(
                "{group}: throughput regressed by {:.2}% ({:.0} vs {:.0} elem/s)",
                d * 100.0,
                new,
                old,
            ));
        }
    } else {
        println!("  throughput: not declared in benchmark target (skip)");
    }
    println!("  wall_time: {:.3}s", current.wall_time_seconds);

    GroupCheckResult {
        max_delta: delta.map(|(d, _, _)| d),
        failure,
    }
}

/// Result of checking a single benchmark group.
struct GroupCheckResult {
    max_delta: Option<f64>,
    failure: Option<String>,
}
