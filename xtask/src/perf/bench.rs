//! Benchmark execution: run `cargo bench` targets and parse output.
//!
//! Criterion 0.8 does not support `--output-format json`; it only supports
//! `"criterion"` (default, ANSI-coloured terminal output) and `"bencher"`
//! (machine-parseable text).  The wrapper below uses the bencher format plus
//! `--noplot` to produce clean stdout.
//!
//! The bencher lines look like:
//!   `test census/parse/hynek_lines_from_html ... bench:    12,345 ns/iter (+/- 1,234)`
//!
//! Wall time is extracted from the bench field; throughput comes from the
//! Criterion-saved `benchmark.json` files in the output directory
//! (`target/criterion/<bench>/...`).

use super::{Cmd, GroupMeasurement};
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// The throughput a bench declares, as Criterion writes it into
/// `target/criterion/<bench>/<group>/<fn>/new/benchmark.json` under the `throughput` key:
/// `{"Elements": 4096}`.
///
/// Only element counts are read. A bench declaring bytes or bits measures something this gate does
/// not compare, so such a file fails to deserialize and the group keeps its wall time with no
/// throughput rather than a number nothing can be compared against.
#[derive(Debug, Deserialize)]
struct DeclaredThroughput {
    #[serde(rename = "Elements")]
    elements: u64,
}

/// The part of that file the gate reads: the group the measurement belongs to and the throughput the
/// bench declared. `function_id` is deliberately absent - the group is the unit the baseline records.
#[derive(Debug, Deserialize)]
struct BenchmarkFile {
    group_id: String,
    throughput: Option<DeclaredThroughput>,
}

/// Run both bench targets through the shell wrapper and parse the output into per-group data.
pub fn run_benchmarks() -> Result<BTreeMap<String, GroupMeasurement>> {
    let mut groups = BTreeMap::new();

    for bench_name in &["core", "pipeline"] {
        let output = Cmd::new("bash")
            .arg("-c")
            .arg(wrapper_script(bench_name))
            .output()
            .with_context(|| format!("running benchmark wrapper for {bench_name}"))?;

        let peak_rss = read_peak_rss(&output);

        let bencher_lines: Vec<&str> = output
            .lines()
            .filter(|l| l.starts_with("test ") && l.contains("bench:"))
            .collect();

        // Collect per-group wall times (average them).
        let mut group_times: BTreeMap<String, Vec<f64>> = BTreeMap::new();
        for line in &bencher_lines {
            let (group, wall_ns) = parse_bencher_line(line)?;
            group_times.entry(group).or_default().push(wall_ns);
        }

        // Parse criterion output directory for throughput declarations.
        let criterion_dir = format!("target/criterion/{bench_name}");
        let throughput_map = read_throughputs(Path::new(&criterion_dir))?;

        // Build group measurements.
        group_measurements(group_times, &throughput_map, peak_rss, &mut groups)?;
    }

    if groups.is_empty() {
        bail!("benchmark output carried no groups; the perf gate would compare nothing");
    }

    Ok(groups)
}

/// Turn one bench target's per-group wall times and declared throughputs into measurements.
///
/// Counts are converted through `u32` so the arithmetic needs no `as` cast, as in
/// `census_crawl::net::FetchStats::useful_records_per_physical_request`: a count past four billion
/// does not occur in a benchmark run, and refusing beats reporting an approximate figure.
fn group_measurements(
    group_times: BTreeMap<String, Vec<f64>>,
    throughput_map: &BTreeMap<String, u64>,
    peak_rss: Option<u64>,
    groups: &mut BTreeMap<String, GroupMeasurement>,
) -> Result<()> {
    for (group, times) in group_times {
        let samples = u32::try_from(times.len())
            .map_err(|_| anyhow::anyhow!("benchmark samples do not fit u32"))?;
        let avg_ns: f64 = times.iter().sum::<f64>() / f64::from(samples);
        let wall_s = avg_ns / 1e9;

        // Throughput: elements/s = declared elements / wall time in seconds.
        let throughput = match throughput_map.get(&group) {
            Some(elements) => {
                let elements = u32::try_from(*elements)
                    .map_err(|_| anyhow::anyhow!("benchmark elements do not fit u32"))?;
                Some(f64::from(elements) / wall_s)
            }
            None => None,
        };

        groups.insert(
            group,
            GroupMeasurement {
                throughput,
                peak_rss_kib: peak_rss,
                wall_time_seconds: wall_s,
            },
        );
    }
    Ok(())
}

/// Extract peak RSS from the wrapper's `peak_rss_kib=` line in the output.
fn read_peak_rss(output: &str) -> Option<u64> {
    output
        .lines()
        .find_map(|l| l.strip_prefix("peak_rss_kib="))
        .and_then(|v| v.parse::<u64>().ok())
}

/// Generate the shell script that wraps `cargo bench` and captures peak RSS via `/usr/bin/time -v`.
///
/// Uses a unique tempfile per invocation (mktemp) to avoid collisions with parallel runs.
/// If `/usr/bin/time` is absent, the peak RSS field is omitted (the baseline records "not measured"
/// with the reason).
fn wrapper_script(bench_name: &str) -> String {
    format!(
        r#"set -e
# Unique temp file per invocation to avoid stomping parallel runs.
CRITERION_OUTPUT=$(mktemp)
trap 'rm -f "$CRITERION_OUTPUT"' EXIT

# Run cargo bench with bencher output (the only machine-parseable format Criterion
# supports).  /usr/bin/time -v captures peak RSS of the Criterion process tree.
if /usr/bin/time -v cargo bench -p census-service --bench {bench_name} -- --output-format bencher --noplot > "$CRITERION_OUTPUT" 2>/tmp/time-output.txt; then
    RSS=$(grep "Maximum resident set size" /tmp/time-output.txt | sed 's/.*: *//')
    if [ -n "$RSS" ]; then
        echo "peak_rss_kib=$RSS"
    else
        echo "peak_rss_kib="
    fi
else
    RSS=$(grep "Maximum resident set size" /tmp/time-output.txt 2>/dev/null | sed 's/.*: *//' || true)
    if [ -n "$RSS" ]; then
        echo "peak_rss_kib=$RSS"
    fi
    cat "$CRITERION_OUTPUT"
    exit 1
fi
cat "$CRITERION_OUTPUT"
rm -f /tmp/time-output.txt
"#,
    )
}

/// Parse a single bencher line into (group name, median wall time in nanoseconds).
///
/// Bencher format (Criterion 0.8):
///   `test <group>/<function> ... bench: <N,NNN> <unit>/iter (+/- <N,NNN>)`
///
/// `<unit>` is one of `ns`, `μs`/`µs`, `ms`, `s`.
pub(crate) fn parse_bencher_line(line: &str) -> Result<(String, f64)> {
    let name = line
        .split_once(" ... bench:")
        .and_then(|(pre, _)| pre.strip_prefix("test "))
        .ok_or_else(|| anyhow::anyhow!("bencher line missing name: {line}"))?;

    let bench_part = line
        .split_once("bench:")
        .and_then(|(_, post)| post.split_once("/iter").map(|(t, _)| t))
        .ok_or_else(|| anyhow::anyhow!("bencher line missing bench time: {line}"))?;

    let bench_part = bench_part.trim();

    let parts: Vec<String> = bench_part.split_whitespace().map(String::from).collect();
    if parts.len() < 2 {
        return Err(anyhow::anyhow!("bencher line malformed: {line}"));
    }
    let (Some(num_str), Some(unit)) = (parts.first(), parts.last()) else {
        return Err(anyhow::anyhow!("bencher line malformed: {line}"));
    };

    let ns = parse_bencher_unit(unit)?;

    let ns_per_iter: f64 = num_str
        .replace(',', "")
        .parse()
        .with_context(|| format!("parsing bench value: {num_str}"))?;

    Ok((name.to_string(), ns_per_iter * ns))
}

/// Convert a bencher unit suffix to nanoseconds-per-unit.
fn parse_bencher_unit(unit: &str) -> Result<f64> {
    match unit {
        "ns" => Ok(1.0),
        "µs" | "μs" => Ok(1_000.0),
        "ms" => Ok(1_000_000.0),
        "s" => Ok(1_000_000_000.0),
        _ => bail!("unknown bencher time unit: {unit}"),
    }
}

/// Walk the Criterion output directory and collect throughput declarations.
///
/// Criterion writes one `benchmark.json` per benchmark function, each containing
/// the BenchmarkId with its group_id and declared throughput.  We return the
/// first throughput per group (all functions in the same group share the group-level
/// `.throughput()` declaration).
fn read_throughputs(dir: &Path) -> Result<BTreeMap<String, u64>> {
    let mut result = BTreeMap::new();

    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return Ok(result),
    };

    for entry_result in read_dir {
        let entry = match entry_result {
            Ok(e) => e,
            Err(_) => continue,
        };
        let is_dir = match entry.file_type() {
            Ok(t) => t.is_dir(),
            Err(_) => continue,
        };
        if !is_dir {
            continue;
        }

        let new_dir = entry.path().join("new");
        let bench_file = new_dir.join("benchmark.json");
        if !bench_file.is_file() {
            continue;
        }

        let content = match std::fs::read_to_string(&bench_file) {
            Ok(c) => c,
            Err(_) => continue,
        };
        // A file this reader cannot place - another throughput kind, or a shape Criterion changed -
        // leaves the group with its wall time and no throughput, never with an invented one.
        let Ok(data) = serde_json::from_str::<BenchmarkFile>(&content) else {
            continue;
        };

        if let Some(throughput) = data.throughput {
            result.entry(data.group_id).or_insert(throughput.elements);
        }
    }

    Ok(result)
}
