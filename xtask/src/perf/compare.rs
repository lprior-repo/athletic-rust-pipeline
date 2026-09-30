use super::env;
use super::{GroupMeasurement, PerfBaseline, Throughput};
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;

pub fn check_environment(baseline: &PerfBaseline) -> Result<()> {
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
    if baseline.metadata.corpus_lines != current_meta.corpus_lines {
        env_warnings.push(format!(
            "Corpus size mismatch: baseline={} current={}",
            baseline.metadata.corpus_lines, current_meta.corpus_lines
        ));
    }

    if !env_warnings.is_empty() {
        println!("\nEnvironment differences detected (comparison may be invalid):");
        for w in &env_warnings {
            println!("  {w}");
        }
        println!(
            "\nbaseline sha: {}  current sha: {}",
            baseline.metadata.sha, current_meta.sha
        );
    }
    Ok(())
}

pub fn validate_tolerance(tolerance: f64) -> Result<()> {
    if !tolerance.is_finite() {
        bail!("tolerance is not finite ({tolerance})");
    }
    if !(0.0..1.0).contains(&tolerance) {
        bail!("tolerance must be in [0, 1) range ({tolerance})");
    }
    Ok(())
}

pub fn check_throughput(
    baseline: &PerfBaseline,
    current_data: &BTreeMap<String, GroupMeasurement>,
    tolerance: f64,
) -> Result<()> {
    validate_tolerance(tolerance)?;
    if baseline.groups.is_empty() || current_data.is_empty() {
        bail!("baseline and current must contain benchmarks");
    }
    validate_ids_match(baseline, current_data)?;
    validate_metrics(baseline, current_data)?;
    let failures = compare_measurements(baseline, current_data, tolerance)?;
    if !failures.is_empty() {
        println!("\nperf check: {} issue(s) detected", failures.len());
        for failure in failures {
            println!("  {failure}");
        }
        bail!("perf check: regression detected");
    }
    println!("\nperf check: no regression detected");
    Ok(())
}

fn compare_measurements(
    baseline: &PerfBaseline,
    current_data: &BTreeMap<String, GroupMeasurement>,
    tolerance: f64,
) -> Result<Vec<String>> {
    let mut failures = Vec::new();
    for (id, current) in current_data {
        let baseline = baseline
            .groups
            .get(id)
            .context("validated benchmark ID disappeared")?;
        if let Comparison::Exceeded(delta) = check_benchmark(id, baseline, current, tolerance)? {
            failures.push(format!("{id}: regression by {:.2}%", delta * 100.0));
        }
    }
    Ok(failures)
}

fn validate_ids_match(
    baseline: &PerfBaseline,
    current_data: &BTreeMap<String, GroupMeasurement>,
) -> Result<()> {
    let mut failures = Vec::new();

    for id in baseline.groups.keys() {
        if !current_data.contains_key(id) {
            failures.push(format!("missing benchmark in current: {id}"));
        }
    }

    for id in current_data.keys() {
        if !baseline.groups.contains_key(id) {
            failures.push(format!("unexpected benchmark in current: {id}"));
        }
    }

    if !failures.is_empty() {
        for f in &failures {
            println!("  {f}");
        }
        bail!("benchmark ID mismatch between baseline and current");
    }

    Ok(())
}

fn validate_metrics(
    baseline: &PerfBaseline,
    current_data: &BTreeMap<String, GroupMeasurement>,
) -> Result<()> {
    for (id, measurement) in baseline.groups.iter() {
        validate_measurement(id, measurement, "baseline")?;
    }

    for (id, measurement) in current_data.iter() {
        validate_measurement(id, measurement, "current")?;
    }

    Ok(())
}

fn validate_measurement(id: &str, measurement: &GroupMeasurement, source: &str) -> Result<()> {
    if !measurement.wall_time_seconds.is_finite() {
        bail!(
            "{source} {id}: wall time is not finite ({})",
            measurement.wall_time_seconds
        );
    }
    if measurement.wall_time_seconds <= 0.0 {
        bail!(
            "{source} {id}: wall time is not positive ({})",
            measurement.wall_time_seconds
        );
    }

    if let Some(throughput) = measurement.throughput {
        let throughput = throughput.value();
        if !throughput.is_finite() {
            bail!("{source} {id}: throughput is not finite ({throughput})");
        }
        if throughput <= 0.0 {
            bail!("{source} {id}: throughput is not positive ({throughput})");
        }
    }

    Ok(())
}

enum Comparison {
    Accepted,
    Exceeded(f64),
}

fn check_benchmark(
    id: &str,
    baseline: &GroupMeasurement,
    current: &GroupMeasurement,
    tolerance: f64,
) -> Result<Comparison> {
    match (baseline.throughput, current.throughput) {
        (Some(Throughput::Elements(old)), Some(Throughput::Elements(new))) => {
            Ok(compare_rate(id, "elements/s", old, new, tolerance))
        }
        (Some(Throughput::Bytes(old)), Some(Throughput::Bytes(new))) => {
            Ok(compare_rate(id, "bytes/s", old, new, tolerance))
        }
        (Some(_), Some(_)) => bail!("{id}: throughput unit changed between elements and bytes"),
        (None, None) => Ok(compare_timing(
            id,
            baseline.wall_time_seconds,
            current.wall_time_seconds,
            tolerance,
        )),
        _ => bail!("{id}: throughput presence changed"),
    }
}

fn compare_rate(id: &str, unit: &str, baseline: f64, current: f64, tolerance: f64) -> Comparison {
    let delta = (baseline - current) / baseline;
    println!(
        "  {id}: baseline {baseline:.0} {unit}, current {current:.0} {unit}, delta {:.2}%",
        delta * 100.0
    );
    if current < baseline * (1.0 - tolerance) {
        Comparison::Exceeded(delta)
    } else {
        Comparison::Accepted
    }
}

fn compare_timing(id: &str, baseline: f64, current: f64, tolerance: f64) -> Comparison {
    let delta = (current - baseline) / baseline;
    println!(
        "  {id}: baseline {baseline:.9}s, current {current:.9}s, delta {:.2}%",
        delta * 100.0
    );
    if current > baseline * (1.0 + tolerance) {
        Comparison::Exceeded(delta)
    } else {
        Comparison::Accepted
    }
}
