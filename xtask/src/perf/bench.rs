//! Benchmark execution: run `cargo bench` targets and parse output.

use super::{Cmd, GroupMeasurement};
use anyhow::{Context, Result};
use std::collections::BTreeMap;

/// Run both bench targets through the shell wrapper and parse the output into per-group data.
pub fn run_benchmarks() -> Result<BTreeMap<String, GroupMeasurement>> {
    let mut groups = BTreeMap::new();

    for bench_name in &["core", "pipeline"] {
        let output = Cmd::new("bash")
            .arg("-c")
            .arg(wrapper_script(bench_name))
            .output()
            .with_context(|| format!("running benchmark wrapper for {bench_name}"))?;

        let mut wall_time: Option<f64> = None;
        let mut peak_rss: Option<u64> = None;

        for line in output.lines() {
            // wall_seconds line: criterion prints `metric=wall_seconds value=12.345 unit=s`
            if let Some(val) = line.strip_prefix("metric=wall_seconds value=") {
                if let Some(val) = val.strip_suffix(" unit=s") {
                    wall_time = Some(
                        val.parse()
                            .with_context(|| format!("parsing wall_seconds: {val}"))?,
                    );
                }
            }
            // peak_rss line: wrapper prints `peak_rss_kib=12345`
            if let Some(val) = line.strip_prefix("peak_rss_kib=") {
                peak_rss = Some(
                    val.parse()
                        .with_context(|| format!("parsing peak_rss_kib: {val}"))?,
                );
            }
            // Benchmark line: `name=...  bench_time=...  throughput=...` or `name=...  bench_time=...`
            if line.starts_with("name=") {
                let group_name = parse_group_name(line);
                let throughput = parse_throughput(line);
                let wall = wall_time.unwrap_or(0.0);
                groups.insert(
                    group_name,
                    GroupMeasurement {
                        throughput,
                        peak_rss_kib: peak_rss,
                        wall_time_seconds: wall,
                    },
                );
            }
        }
    }

    Ok(groups)
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

# Run cargo bench with criterion's JSON reporter. /usr/bin/time -v captures peak RSS of the
# Criterion process tree (Maximum resident set size in KiB) from the child, not the shell.
if /usr/bin/time -v cargo bench -p census-service --bench {bench_name} -- --output-format json > "$CRITERION_OUTPUT" 2>/tmp/time-output.txt; then
    # Parse Maximum resident set size from /usr/bin/time -v output.
    RSS=$(grep "Maximum resident set size" /tmp/time-output.txt | sed 's/.*: *//')
    if [ -n "$RSS" ]; then
        echo "peak_rss_kib=$RSS"
    else
        echo "peak_rss_kib="
    fi
else
    # cargo bench exited non-zero — still try to capture RSS if /usr/bin/time produced it.
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

/// Extract group name from a benchmark output line (e.g. `name=census/parse`).
fn parse_group_name(line: &str) -> String {
    line.split_whitespace()
        .find_map(|part| part.strip_prefix("name="))
        .unwrap_or("")
        .to_string()
}

/// Extract throughput value from a benchmark output line (e.g. `throughput=1234.5 elem/s`).
fn parse_throughput(line: &str) -> Option<f64> {
    for part in line.split_whitespace() {
        if let Some(v) = part.strip_prefix("throughput=") {
            return v.split('/').next().and_then(|v| v.parse().ok());
        }
    }
    None
}
