mod metadata;
pub(super) mod runtime;

use super::{GroupMeasurement, Throughput};
use anyhow::{bail, Context, Result};
use std::borrow::Cow;
use std::collections::{btree_map::Entry, BTreeMap};

pub fn run_benchmarks() -> Result<BTreeMap<String, GroupMeasurement>> {
    let mut groups = BTreeMap::new();
    for name in ["core", "pipeline"] {
        let executable = runtime::compile(name)?;
        let directory = tempfile::tempdir().context("creating benchmark measurement directory")?;
        let (output, rss) = runtime::measure(&executable, directory.path())?;
        let declarations = metadata::read(&directory.path().join("criterion"))?;
        for (id, measurement) in parse_measurement(&output, &declarations, rss)? {
            match groups.entry(id) {
                Entry::Vacant(entry) => {
                    entry.insert(measurement);
                }
                Entry::Occupied(entry) => {
                    bail!("duplicate benchmark ID across targets: {}", entry.key())
                }
            }
        }
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
    let mut fields = timing.split_whitespace();
    let number = fields.next().context("missing benchmark timing")?;
    let multiplier = match fields.next() {
        Some("ns") => 1.0,
        Some("µs" | "μs") => 1_000.0,
        Some("ms") => 1_000_000.0,
        Some("s") => 1_000_000_000.0,
        _ => bail!("unsupported benchmark time unit: {line}"),
    };
    if fields.next().is_some() {
        bail!("unexpected benchmark timing fields: {line}");
    }
    let number = if number.contains(',') {
        Cow::Owned(number.replace(',', ""))
    } else {
        Cow::Borrowed(number)
    };
    let nanos = number.parse::<f64>().context("invalid benchmark timing")? * multiplier;
    if !nanos.is_finite() || nanos <= 0.0 {
        bail!("benchmark timing must be finite and positive: {line}");
    }
    Ok((id.to_owned(), nanos))
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
        let seconds = nanos / 1e9;
        if seconds <= 0.0 {
            bail!("benchmark timing underflow for {id}");
        }
        let throughput = amount
            .map(|value| -> Result<_> {
                let numeric = serde_json::Number::from(value.value())
                    .as_f64()
                    .context("Criterion throughput amount cannot be converted")?;
                Ok(value.map(|_| numeric / seconds))
            })
            .transpose()?;
        if throughput.is_some_and(|value| !value.value().is_finite() || value.value() <= 0.0) {
            bail!("invalid benchmark throughput for {id}");
        }
        let measurement = GroupMeasurement {
            throughput,
            peak_rss_kib: rss,
            wall_time_seconds: seconds,
        };
        match result.entry(id) {
            Entry::Vacant(entry) => {
                entry.insert(measurement);
            }
            Entry::Occupied(entry) => bail!("duplicate benchmark result for {}", entry.key()),
        }
    }
    if result.is_empty() || !result.keys().eq(declarations.keys()) {
        bail!("Criterion results and metadata must contain the same nonempty benchmark IDs");
    }
    Ok(result)
}

#[cfg(test)]
#[path = "bench/tests.rs"]
mod tests;
