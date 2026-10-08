use super::env;
use super::{GroupMeasurement, Meta, PerfBaseline, Throughput};
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;

pub(super) const CAPTURE_EXPORT_WORKLOAD: &str =
    "pipeline/capture_export/captured_live_wiaa_co2027";
pub(super) const CAPTURE_EXPORT_TIMING_SCOPE: &str = "capture-publication-independent-readback/v1";
const REQUIRED_WORKLOADS: &[&str] = &[CAPTURE_EXPORT_WORKLOAD];

pub fn check_environment(baseline: &PerfBaseline) -> Result<()> {
    compare_environment(&baseline.metadata, &env::build_meta()?)
}

pub(super) fn compare_environment(baseline: &Meta, current: &Meta) -> Result<()> {
    validate_environment(baseline)?;
    validate_environment(current)?;
    for (label, old, new) in [
        ("CPU", baseline.cpu.as_str(), current.cpu.as_str()),
        ("rustc", baseline.rustc.as_str(), current.rustc.as_str()),
    ] {
        if old != new {
            bail!("{label} mismatch: baseline={old} current={new}");
        }
    }
    if baseline.cores != current.cores {
        bail!(
            "Core count mismatch: baseline={} current={}",
            baseline.cores,
            current.cores
        );
    }
    if baseline.corpus_lines != current.corpus_lines {
        bail!(
            "Corpus size mismatch: baseline={} current={}",
            baseline.corpus_lines,
            current.corpus_lines
        );
    }
    if baseline.corpus_sha256 != current.corpus_sha256 {
        bail!(
            "Corpus digest mismatch: baseline={} current={}",
            baseline.corpus_sha256,
            current.corpus_sha256
        );
    }
    Ok(())
}

fn validate_environment(meta: &Meta) -> Result<()> {
    if meta.cpu.trim().is_empty() || meta.cpu.trim() == "unknown" {
        bail!("CPU identity is unavailable");
    }
    if meta.rustc.trim().is_empty() || meta.rustc.trim() == "unknown" {
        bail!("rustc identity is unavailable");
    }
    if meta.cores == 0 || meta.corpus_lines == 0 {
        bail!("core count and corpus size must be positive");
    }
    if meta.corpus_sha256.len() != 64
        || !meta
            .corpus_sha256
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        bail!("fixture corpus SHA256 is unavailable or invalid");
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
    validate_required_workloads(&baseline.groups, "baseline")?;
    validate_required_workloads(current_data, "current")?;
    validate_ids_match(baseline, current_data)?;
    validate_metrics(baseline, current_data)?;
    let failures = compare_measurements(baseline, current_data, tolerance)?;
    if !failures.is_empty() {
        bail!("perf check: regression detected\n{}", failures.join("\n"));
    }
    println!("\nperf check: no regression detected");
    Ok(())
}

pub(super) fn validate_required_workloads(
    groups: &BTreeMap<String, GroupMeasurement>,
    source: &str,
) -> Result<()> {
    for id in REQUIRED_WORKLOADS {
        if !groups.contains_key(*id) {
            bail!("{source}: required workload was not measured: {id}");
        }
    }
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
        compare_benchmark(id, baseline, current, tolerance, &mut failures)?;
    }
    Ok(failures)
}

fn validate_ids_match(
    baseline: &PerfBaseline,
    current_data: &BTreeMap<String, GroupMeasurement>,
) -> Result<()> {
    if !baseline.groups.keys().eq(current_data.keys()) {
        bail!("benchmark ID mismatch between baseline and current");
    }
    Ok(())
}

fn validate_metrics(
    baseline: &PerfBaseline,
    current_data: &BTreeMap<String, GroupMeasurement>,
) -> Result<()> {
    validate_baseline(baseline)?;
    for (id, measurement) in current_data {
        validate_measurement(id, measurement, "current")?;
    }
    Ok(())
}

pub(super) fn validate_baseline(baseline: &PerfBaseline) -> Result<()> {
    if baseline.groups.is_empty() {
        bail!("baseline must contain benchmarks");
    }
    for (id, measurement) in &baseline.groups {
        validate_measurement(id, measurement, "baseline")?;
    }
    validate_required_workloads(&baseline.groups, "baseline")?;
    Ok(())
}

pub(super) fn validate_measurement(
    id: &str,
    measurement: &GroupMeasurement,
    source: &str,
) -> Result<()> {
    positive_metric(id, "wall time", measurement.wall_time_seconds)
        .with_context(|| source.to_owned())?;
    let tail = measurement
        .tail_time_seconds
        .with_context(|| format!("{source} {id}: missing tail time"))?;
    positive_metric(id, "tail time", tail)?;
    let throughput = measurement
        .throughput
        .with_context(|| format!("{source} {id}: missing throughput"))?;
    positive_metric(id, "throughput", throughput.value())?;
    for (label, value) in [
        ("peak RSS", measurement.peak_rss_kib),
        ("allocation count", measurement.allocation_count),
        ("allocated bytes", measurement.allocated_bytes),
    ] {
        let value = value.with_context(|| format!("{source} {id}: missing {label}"))?;
        if value == 0 {
            bail!("{source} {id}: {label} must be positive");
        }
    }
    if id == CAPTURE_EXPORT_WORKLOAD
        && measurement.timing_scope.as_deref() != Some(CAPTURE_EXPORT_TIMING_SCOPE)
    {
        bail!("{source} {id}: incompatible timing scope; required {CAPTURE_EXPORT_TIMING_SCOPE}");
    }
    Ok(())
}

fn positive_metric(id: &str, label: &str, value: f64) -> Result<()> {
    if !value.is_finite() || value <= 0.0 {
        bail!("{id}: {label} must be finite and positive ({value})");
    }
    Ok(())
}

fn compare_benchmark(
    id: &str,
    baseline: &GroupMeasurement,
    current: &GroupMeasurement,
    tolerance: f64,
    failures: &mut Vec<String>,
) -> Result<()> {
    compare_cost(
        id,
        "wall time",
        (baseline.wall_time_seconds, current.wall_time_seconds),
        tolerance,
        failures,
    );
    compare_cost(
        id,
        "tail time",
        (
            required_float(baseline.tail_time_seconds)?,
            required_float(current.tail_time_seconds)?,
        ),
        tolerance,
        failures,
    );
    compare_memory(id, baseline, current, tolerance, failures)?;
    match (baseline.throughput, current.throughput) {
        (Some(Throughput::Elements(old)), Some(Throughput::Elements(new)))
        | (Some(Throughput::Bytes(old)), Some(Throughput::Bytes(new))) => {
            if new < old * (1.0 - tolerance) {
                failures.push(format!(
                    "{id}: throughput regression: baseline={old} current={new}"
                ));
            }
        }
        (Some(_), Some(_)) => bail!("{id}: throughput unit changed between elements and bytes"),
        _ => bail!("{id}: missing throughput"),
    }
    Ok(())
}

fn compare_memory(
    id: &str,
    baseline: &GroupMeasurement,
    current: &GroupMeasurement,
    tolerance: f64,
    failures: &mut Vec<String>,
) -> Result<()> {
    for (label, old, new) in [
        ("peak RSS", baseline.peak_rss_kib, current.peak_rss_kib),
        (
            "allocation count",
            baseline.allocation_count,
            current.allocation_count,
        ),
        (
            "allocated bytes",
            baseline.allocated_bytes,
            current.allocated_bytes,
        ),
    ] {
        compare_cost(
            id,
            label,
            (numeric(old)?, numeric(new)?),
            tolerance,
            failures,
        );
    }
    Ok(())
}

fn required_float(value: Option<f64>) -> Result<f64> {
    value.context("validated tail measurement disappeared")
}

fn numeric(value: Option<u64>) -> Result<f64> {
    serde_json::Number::from(value.context("validated memory measurement disappeared")?)
        .as_f64()
        .context("memory measurement cannot be converted")
}

fn compare_cost(
    id: &str,
    label: &str,
    (baseline, current): (f64, f64),
    tolerance: f64,
    failures: &mut Vec<String>,
) {
    if current > baseline * (1.0 + tolerance) {
        failures.push(format!(
            "{id}: {label} regression: baseline={baseline} current={current}"
        ));
    }
}
