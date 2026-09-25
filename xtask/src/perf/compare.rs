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
    // A current run that produced no groups means nothing was measured; refuse the gate.
    if current_data.is_empty() {
        bail!("perf check: current run produced no groups; cannot compare against baseline");
    }

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

    // Every baseline group must appear in the current run; a missing group means the comparison
    // has blind spots and must refuse rather than silently pass.
    for baseline_group in baseline.groups.keys() {
        if !current_data.contains_key(baseline_group) {
            failures.push(format!(
                "{baseline_group}: present in baseline but absent from current run"
            ));
        }
    }

    if !failures.is_empty() {
        println!("\nperf check: {} issue(s) detected", failures.len());
        for f in &failures {
            println!("  {f}");
        }
        bail!(
            "perf check: regression or missing data detected (max delta {:.2}%)",
            max_delta * 100.0
        );
    }

    println!("\nperf check: no regression detected");
    Ok(())
}

/// Compute the throughput delta between baseline and current values.
/// Returns `Err(failure_msg)` if either value is non-finite, `Ok(None)` if
/// either value is `None` (throughput not declared), and `Ok(Some((delta, old, new)))`
/// when both values are present and finite.
fn compute_delta(
    group: &str,
    baseline: Option<f64>,
    current: Option<f64>,
) -> Result<Option<(f64, f64, f64)>, String> {
    let (Some(old), Some(new)) = (baseline, current) else {
        return Ok(None);
    };
    if !old.is_finite() {
        return Err(format!(
            "{group}: baseline throughput is non-finite ({old})"
        ));
    }
    if !new.is_finite() {
        return Err(format!("{group}: current throughput is non-finite ({new})"));
    }
    let d = (old - new) / old;
    Ok(Some((d, old, new)))
}

/// Check a single group's throughput against the baseline.
fn check_group<'a>(
    groups: &'a BTreeMap<String, GroupMeasurement>,
    group: &str,
    current: &'a GroupMeasurement,
    tolerance: f64,
) -> GroupCheckResult {
    println!("group: {group}");

    // A group the baseline never recorded cannot be compared. Report it as a failure the operator
    // can act on: aborting the comparison here hid every other group's result behind a panic.
    let Some(baseline) = groups.get(group) else {
        return GroupCheckResult {
            max_delta: None,
            failure: Some(format!(
                "{group}: baseline has no measurement for this group — run `perf record` first"
            )),
        };
    };
    // Check throughput delta; handles absent values (skip) and non-finite values (fail).
    let mut failure: Option<String> = None;
    let delta = match compute_delta(group, baseline.throughput, current.throughput) {
        Ok(Some((d, old, new))) => {
            println!("  throughput: {:.2}%", d * 100.0);
            if d > tolerance {
                failure = Some(format!(
                    "{group}: throughput regressed by {:.2}% ({:.0} vs {:.0} elem/s)",
                    d * 100.0,
                    new,
                    old,
                ));
            }
            Some((d, old, new))
        }
        Ok(None) => {
            println!("  throughput: not declared in benchmark target (skip)");
            None
        }
        Err(e) => {
            failure = Some(e);
            None
        }
    };
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
