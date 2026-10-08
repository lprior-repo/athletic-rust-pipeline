mod memory;
mod metadata;
pub(super) mod runtime;
mod tail;

use super::{GroupMeasurement, Throughput};
use anyhow::{bail, Context, Result};
use std::borrow::Cow;
use std::collections::{btree_map::Entry, BTreeMap};

pub fn run_benchmarks() -> Result<BTreeMap<String, GroupMeasurement>> {
    let mut groups = BTreeMap::new();
    for name in ["core", "pipeline", "capture_export"] {
        for (id, measurement) in measure_target(name)? {
            insert_measurement(&mut groups, id, measurement)?;
        }
    }
    super::compare::validate_required_workloads(&groups, "recorded")?;
    Ok(groups)
}

fn measure_target(name: &str) -> Result<BTreeMap<String, GroupMeasurement>> {
    let executable = runtime::compile(name)?;
    let directory = tempfile::tempdir().context("creating benchmark measurement directory")?;
    let (output, rss) = runtime::measure(&executable, directory.path())?;
    let root = directory.path().join("criterion");
    let declarations = metadata::read(&root)?;
    let tails = tail::read(&root)?;
    let mut groups = parse_measurement(&output, &declarations, rss)?;
    if !groups.keys().eq(tails.keys()) {
        bail!("Criterion results and tail samples must contain the same benchmark IDs");
    }
    for (id, measurement) in &mut groups {
        let memory = memory::measure(&executable, directory.path(), id)?;
        measurement.peak_rss_kib = Some(memory.peak_rss_kib);
        measurement.allocation_count = Some(memory.allocation_count);
        measurement.allocated_bytes = Some(memory.allocated_bytes);
        if name == "capture_export" && id == super::compare::CAPTURE_EXPORT_WORKLOAD {
            measurement.timing_scope = Some(super::compare::CAPTURE_EXPORT_TIMING_SCOPE.to_owned());
        }
        measurement.tail_time_seconds = Some(
            *tails
                .get(id)
                .with_context(|| format!("missing tail samples for {id}"))?,
        );
        super::compare::validate_measurement(id, measurement, "recorded")?;
    }
    Ok(groups)
}

pub(crate) fn parse_bencher_line(line: &str) -> Result<(String, f64)> {
    let (head, tail) = line
        .split_once(" ... bench:")
        .ok_or_else(|| anyhow::anyhow!("invalid bencher output: {line}"))?;
    let id = head
        .strip_prefix("test ")
        .filter(|id| !id.is_empty())
        .ok_or_else(|| anyhow::anyhow!("missing benchmark ID: {line}"))?;
    let (timing, _) = tail
        .split_once("/iter")
        .ok_or_else(|| anyhow::anyhow!("missing iteration unit: {line}"))?;
    Ok((id.to_owned(), parse_timing(timing)?))
}

fn parse_timing(timing: &str) -> Result<f64> {
    let mut fields = timing.split_whitespace();
    let number = fields.next().context("missing benchmark timing")?;
    let multiplier = match fields.next() {
        Some("ns") => 1.0,
        Some("µs" | "μs") => 1_000.0,
        Some("ms") => 1_000_000.0,
        Some("s") => 1_000_000_000.0,
        _ => bail!("unsupported benchmark time unit: {timing}"),
    };
    if fields.next().is_some() {
        bail!("unexpected benchmark timing fields: {timing}");
    }
    let number = if number.contains(',') {
        Cow::Owned(number.replace(',', ""))
    } else {
        Cow::Borrowed(number)
    };
    let nanos = number.parse::<f64>().context("invalid benchmark timing")? * multiplier;
    if !nanos.is_finite() || nanos <= 0.0 {
        bail!("benchmark timing must be finite and positive: {timing}");
    }
    Ok(nanos)
}

fn parse_measurement(
    output: &str,
    declarations: &BTreeMap<String, Option<Throughput<u64>>>,
    rss: Option<u64>,
) -> Result<BTreeMap<String, GroupMeasurement>> {
    let mut result = BTreeMap::new();
    for line in output
        .lines()
        .filter(|line| line.starts_with("test ") && line.contains("bench:"))
    {
        let (id, nanos) = parse_bencher_line(line)?;
        let amount = declarations
            .get(&id)
            .with_context(|| format!("missing Criterion metadata for {id}"))?;
        let amount = amount
            .as_ref()
            .with_context(|| format!("benchmark {id} must declare a throughput amount"))?;
        if amount.value() == 0 {
            bail!("benchmark {id} throughput must be positive");
        }
        insert_measurement(&mut result, id, timing_measurement(nanos, *amount, rss)?)?;
    }
    if result.is_empty() || !result.keys().eq(declarations.keys()) {
        bail!("Criterion results and metadata must contain the same nonempty benchmark IDs");
    }
    Ok(result)
}

fn timing_measurement(
    nanos: f64,
    amount: Throughput<u64>,
    rss: Option<u64>,
) -> Result<GroupMeasurement> {
    let seconds = nanos / 1e9;
    if seconds <= 0.0 {
        bail!("benchmark timing underflow");
    }
    let numeric = serde_json::Number::from(amount.value())
        .as_f64()
        .context("Criterion throughput amount cannot be converted")?;
    let throughput = amount.map(|_| numeric / seconds);
    if !throughput.value().is_finite() || throughput.value() <= 0.0 {
        bail!("invalid benchmark throughput");
    }
    Ok(GroupMeasurement {
        throughput: Some(throughput),
        peak_rss_kib: rss,
        allocation_count: None,
        allocated_bytes: None,
        tail_time_seconds: None,
        timing_scope: None,
        wall_time_seconds: seconds,
    })
}

fn insert_measurement(
    groups: &mut BTreeMap<String, GroupMeasurement>,
    id: String,
    measurement: GroupMeasurement,
) -> Result<()> {
    match groups.entry(id) {
        Entry::Vacant(entry) => {
            entry.insert(measurement);
        }
        Entry::Occupied(entry) => bail!("duplicate benchmark result for {}", entry.key()),
    }
    Ok(())
}

#[cfg(test)]
#[path = "bench/tests.rs"]
mod tests;
