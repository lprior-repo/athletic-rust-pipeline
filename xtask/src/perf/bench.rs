mod dhat;
mod memory;
mod metadata;
mod policy;
pub(super) mod runtime;
mod tail;

#[cfg(test)]
mod tests;

use super::{GroupMeasurement, Throughput};
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;

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
    let mut trials = Vec::new();
    for trial in runtime::measure_trials(&executable, directory.path())? {
        trials.push(read_trial(name, &trial)?);
    }
    let mut groups = select_timing(name, trials)?;
    for (id, measurement) in &mut groups {
        let memory = memory::measure(&executable, directory.path(), id)?;
        measurement.peak_rss_kib = Some(memory.peak_rss_kib);
        measurement.allocation_count = Some(memory.allocation_count);
        measurement.allocated_bytes = Some(memory.allocated_bytes);
        measurement.timing_scope = Some(super::scope::expected(id).to_owned());
        super::compare::validate_measurement(id, measurement, "recorded")?;
    }
    Ok(groups)
}

fn read_trial(target: &str, trial: &runtime::Trial) -> Result<BTreeMap<String, GroupMeasurement>> {
    let declarations = metadata::read(&trial.criterion)
        .with_context(|| format!("{target}: reading trial criterion declarations"))?;
    let tails = tail::read(&trial.criterion)
        .with_context(|| format!("{target}: reading trial criterion tail samples"))?;
    parse_measurement(&trial.output, &declarations, &tails)
        .with_context(|| format!("{target}: parsing trial results"))
}

fn select_timing(
    target: &str,
    trials: Vec<BTreeMap<String, GroupMeasurement>>,
) -> Result<BTreeMap<String, GroupMeasurement>> {
    let mut remaining = trials.into_iter();
    let mut selected = match remaining.next() {
        Some(first) => first,
        None => bail!("{target}: no benchmark trial was measured"),
    };
    if selected.is_empty() {
        bail!("{target}: benchmark trials measured no benchmark");
    }
    for trial in remaining {
        if trial.len() != selected.len() {
            bail!("{target}: benchmark trials must publish the same benchmark IDs");
        }
        for (id, measurement) in trial {
            let current = selected.get_mut(&id).ok_or_else(|| {
                anyhow::anyhow!("{target}: benchmark trials must publish the same benchmark IDs")
            })?;
            let policy = policy::declared(&id)?;
            if policy.is_faster(measurement.wall_time_seconds, current.wall_time_seconds) {
                *current = measurement;
            }
        }
    }
    Ok(selected)
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

pub(crate) fn parse_measurement(
    output: &str,
    declarations: &BTreeMap<String, Option<Throughput<u64>>>,
    tails: &BTreeMap<String, f64>,
) -> Result<BTreeMap<String, GroupMeasurement>> {
    let mut measured = BTreeMap::new();
    for line in output.lines().filter(|line| line.starts_with("test ")) {
        let (id, duration) = parse_bencher_line(line)?;
        let declaration = declarations.get(&id).ok_or_else(|| {
            anyhow::anyhow!("trial result {id} has no criterion declaration in this trial")
        })?;
        let tail_time_seconds = tails.get(&id).copied().ok_or_else(|| {
            anyhow::anyhow!("trial result {id} has no tail samples in this trial")
        })?;
        let wall_time_seconds = duration / 1_000_000_000.0;
        let throughput = match declaration {
            Some(declared) => {
                let rate = measured_rate(declared.value(), wall_time_seconds)?;
                Some(declared.map(|_| rate))
            }
            None => None,
        };
        if measured
            .insert(
                id,
                GroupMeasurement {
                    wall_time_seconds,
                    tail_time_seconds: Some(tail_time_seconds),
                    throughput,
                    timing_scope: None,
                    peak_rss_kib: None,
                    allocation_count: None,
                    allocated_bytes: None,
                },
            )
            .is_some()
        {
            bail!("bencher output has duplicate benchmark results");
        }
    }
    verify_declarations(&measured, declarations, tails)?;
    Ok(measured)
}

fn verify_declarations(
    measured: &BTreeMap<String, GroupMeasurement>,
    declarations: &BTreeMap<String, Option<Throughput<u64>>>,
    tails: &BTreeMap<String, f64>,
) -> Result<()> {
    if measured.is_empty() {
        bail!("bencher output has no benchmark results");
    }
    for id in measured.keys() {
        let declared = declarations
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("measured benchmark {id} is not declared"))?;
        let declared = declared
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("measured benchmark {id} declares no throughput"))?;
        if declared.value() == 0 {
            bail!("measured benchmark {id} declares a zero throughput");
        }
        if !tails.contains_key(id) {
            bail!("measured benchmark {id} has no tail samples");
        }
    }
    if measured.len() != declarations.len() || measured.len() != tails.len() {
        bail!("trial results and criterion samples must describe the same benchmarks");
    }
    Ok(())
}

fn measured_rate(amount: u64, seconds: f64) -> Result<f64> {
    let amount = serde_json::Number::from(amount)
        .as_f64()
        .context("benchmark throughput amount is not representable")?;
    Ok(amount / seconds)
}

fn parse_timing(raw: &str) -> Result<f64> {
    let text = raw.trim();
    let (number, multiplier) = match ["ns", "µs", "μs", "ms", "s"]
        .into_iter()
        .find_map(|unit| text.strip_suffix(unit).map(|head| (head.trim_end(), unit)))
    {
        Some(parts) => parts,
        None => bail!("missing time unit: {raw}"),
    };
    let (digits, sign) = number
        .strip_prefix('-')
        .map_or_else(|| (number, 1), |rest| (rest, -1));
    let value = digits
        .chars()
        .filter(|character| *character != ',')
        .collect::<String>()
        .parse::<f64>()
        .map_err(|error| anyhow::anyhow!("invalid bencher time {raw}: {error}"))?;
    let multiplier = match multiplier {
        "ns" => 1.0,
        "µs" | "μs" => 1_000.0,
        "ms" => 1_000_000.0,
        "s" => 1_000_000_000.0,
        _ => bail!("unsupported bencher time unit: {raw}"),
    };
    let number = f64::from(sign) * value * multiplier;
    if !number.is_finite() || number <= 0.0 {
        bail!("bencher time must be positive: {raw}");
    }
    Ok(number)
}

fn insert_measurement(
    groups: &mut BTreeMap<String, GroupMeasurement>,
    id: String,
    measurement: GroupMeasurement,
) -> Result<()> {
    if groups.insert(id.clone(), measurement).is_some() {
        bail!("duplicate benchmark result {id}");
    }
    Ok(())
}
