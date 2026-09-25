//! Fixture-driven unit tests for the benchmark parser.
//!
//! These tests exercise `parse_bencher_line` and the empty-stream error path
//! without running actual benchmarks.

use crate::perf::bench::parse_bencher_line;

/// Realistic Criterion bencher output for two benchmarks in a group.
///
/// Criterion 0.8 bencher format:
///   `test <group>/<function> ... bench: <N,NNN> <unit>/iter (+/- <N,NNN>)`
const BENCHER_SAMPLE: &str = "test census/parse/hynek_lines_from_html ... bench:   12,345 ns/iter (+/- 1,234)\ntest census/parse/hynek_parse ... bench:   23,456 ns/iter (+/- 2,345)\n";

/// Bencher output with no throughput declared (wall time should still parse).
const BENCHER_NO_THROUGHPUT: &str =
    "test census/school_index/normalize_label ... bench:    1,234 ns/iter (+/- 123)\n";

/// Garbage output that contains no valid bencher lines.
const GARBAGE_OUTPUT: &str =
    "this is not criterion output\nname=something\nmetric=wall_seconds value=1.0\n";

/// Test that a normal bencher line with nanoseconds parses correctly.
#[test]
fn parse_bencher_line_normal_ns() {
    let line = "test census/parse/hynek_lines_from_html ... bench:   12,345 ns/iter (+/- 1,234)";
    let (name, ns) = parse_bencher_line(line).expect("should parse");
    assert_eq!(name, "census/parse/hynek_lines_from_html");
    assert_eq!(ns, 12_345.0);
}

/// Test that a bencher line with the second benchmark parses correctly.
#[test]
fn parse_bencher_line_second_benchmark() {
    let line = "test census/parse/hynek_parse ... bench:   23,456 ns/iter (+/- 2,345)";
    let (name, ns) = parse_bencher_line(line).expect("should parse");
    assert_eq!(name, "census/parse/hynek_parse");
    assert_eq!(ns, 23_456.0);
}

/// Test that microseconds parse correctly (both μ and µ variants).
#[test]
fn parse_bencher_line_microseconds() {
    for unit in &["μs", "µs"] {
        let line = format!("test group/fn ... bench: 123 {unit}/iter (+/- 10)");
        let (name, ns) = parse_bencher_line(&line).expect("should parse");
        assert_eq!(name, "group/fn");
        assert_eq!(ns, 123_000.0);
    }
}

/// Test that milliseconds parse correctly.
#[test]
fn parse_bencher_line_milliseconds() {
    let line = "test group/fn ... bench: 5 ms/iter (+/- 1)";
    let (name, ns) = parse_bencher_line(line).expect("should parse");
    assert_eq!(name, "group/fn");
    assert_eq!(ns, 5_000_000.0);
}

/// Test that seconds parse correctly.
#[test]
fn parse_bencher_line_seconds() {
    let line = "test slow_bench ... bench: 2.5 s/iter (+/- 0.1)";
    let (name, ns) = parse_bencher_line(line).expect("should parse");
    assert_eq!(name, "slow_bench");
    assert!((ns - 2_500_000_000.0).abs() < 1.0);
}

/// Test that a bencher line with no "bench:" field is rejected.
#[test]
fn parse_bencher_line_missing_bench() {
    let line = "test group/fn something wrong";
    let err = parse_bencher_line(line).expect_err("should error");
    assert!(err.to_string().contains("bench"));
}

/// Test that a garbled line (no "test " prefix) is rejected.
#[test]
fn parse_bencher_line_empty() {
    let err = parse_bencher_line("").expect_err("should error");
    assert!(err.to_string().contains("name"));
}

/// Garbage output yields zero bencher lines — caller detects this as an error.
#[test]
fn garbled_output_is_error() {
    let bencher_lines: Vec<&str> = GARBAGE_OUTPUT
        .lines()
        .filter(|l| l.starts_with("test ") && l.contains("bench:"))
        .collect();
    assert!(
        bencher_lines.is_empty(),
        "garbage should yield zero bencher lines"
    );
}

/// Empty output yields zero bencher lines.
#[test]
fn empty_output_yields_no_lines() {
    let bencher_lines: Vec<&str> = ""
        .lines()
        .filter(|l| l.starts_with("test ") && l.contains("bench:"))
        .collect();
    assert!(
        bencher_lines.is_empty(),
        "empty output should yield zero lines"
    );
}

/// Parse a multi-line bencher stream (simulates real output).
#[test]
fn parse_multi_line_stream() {
    let mut parsed = Vec::new();
    for line in BENCHER_SAMPLE.lines() {
        let (name, ns) = parse_bencher_line(line).expect("line should parse");
        parsed.push((name, ns));
    }
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].0, "census/parse/hynek_lines_from_html");
    assert_eq!(parsed[0].1, 12_345.0);
    assert_eq!(parsed[1].0, "census/parse/hynek_parse");
    assert_eq!(parsed[1].1, 23_456.0);
}

/// Parse a bencher line that has no throughput (just wall time).
#[test]
fn parse_bencher_no_throughput() {
    let line = BENCHER_NO_THROUGHPUT.trim();
    let (name, ns) = parse_bencher_line(line).expect("should parse");
    assert_eq!(name, "census/school_index/normalize_label");
    assert_eq!(ns, 1_234.0);
}

/// Large seconds value parses correctly.
#[test]
fn parse_bencher_line_large_seconds() {
    let line = "test massive_group/massive_bench ... bench: 123.456 s/iter (+/- 12.345)";
    let (name, ns) = parse_bencher_line(line).expect("should parse");
    assert_eq!(name, "massive_group/massive_bench");
    assert!((ns - 123_456_000_000.0).abs() < 1.0);
}

// ---------------------------------------------------------------------------
// Tests for compare.rs: missing evidence, empty maps, and non-finite values.
// These tests must fail if the corresponding guard is removed from compare.rs.
// ---------------------------------------------------------------------------

use crate::perf::compare::check_throughput;
use crate::perf::{GroupMeasurement, Meta, PerfBaseline};
use std::collections::BTreeMap;

fn make_baseline(
    groups: BTreeMap<String, GroupMeasurement>,
    cpu: &str,
    cores: u32,
    rustc: &str,
    sha: &str,
    corpus_lines: u64,
) -> PerfBaseline {
    PerfBaseline {
        metadata: Meta {
            cpu: cpu.to_string(),
            cores,
            rustc: rustc.to_string(),
            sha: sha.to_string(),
            corpus_lines,
        },
        check_reason: None,
        groups,
    }
}

fn group_measurement(throughput: Option<f64>, wall_time: f64) -> GroupMeasurement {
    GroupMeasurement {
        throughput,
        peak_rss_kib: None,
        wall_time_seconds: wall_time,
    }
}

/// Test that an empty current map is rejected — the gate must refuse when there
/// is nothing to compare against the baseline.
///
/// This test fails if the `current_data.is_empty()` check is removed from
/// `check_throughput` in compare.rs.
#[test]
fn empty_current_map_is_rejected() {
    let mut baseline_groups = BTreeMap::new();
    baseline_groups.insert(
        "census/parse".to_string(),
        group_measurement(Some(1000.0), 1.5),
    );
    let baseline = make_baseline(baseline_groups, "x86_64", 8, "1.75", "abc123", 42);

    let current: BTreeMap<String, GroupMeasurement> = BTreeMap::new();

    let result = check_throughput(&baseline, &current, 0.05);
    assert!(
        result.is_err(),
        "empty current map should fail the comparison"
    );
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("no groups"),
        "error message should mention empty groups; got: {err}"
    );
}

/// Test that a missing baseline group (present in baseline but absent from current)
/// is rejected — the gate must not silently pass when the current run does not
/// exercise every baseline group.
///
/// This test fails if the baseline-key check in `check_throughput` is removed
/// from compare.rs.
#[test]
fn missing_baseline_group_is_rejected() {
    let mut baseline_groups = BTreeMap::new();
    baseline_groups.insert(
        "census/parse".to_string(),
        group_measurement(Some(1000.0), 1.5),
    );
    baseline_groups.insert(
        "pipeline/result_file".to_string(),
        group_measurement(Some(500.0), 2.0),
    );
    let baseline = make_baseline(baseline_groups, "x86_64", 8, "1.75", "abc123", 42);

    // Current run only produced "census/parse", missing "pipeline/result_file".
    let mut current = BTreeMap::new();
    current.insert(
        "census/parse".to_string(),
        group_measurement(Some(1000.0), 1.5),
    );

    let result = check_throughput(&baseline, &current, 0.05);
    assert!(
        result.is_err(),
        "missing baseline group should fail the comparison"
    );
}

/// Test that a NaN throughput measurement is rejected — IEEE NaN comparisons
/// are always false, so a NaN value would silently pass the tolerance check
/// without this guard.
///
/// This test fails if the `is_finite()` check is removed from `check_group`
/// in compare.rs.
#[test]
fn nan_throughput_is_rejected() {
    let mut baseline_groups = BTreeMap::new();
    baseline_groups.insert(
        "census/parse".to_string(),
        group_measurement(Some(f64::NAN), 1.5),
    );
    let baseline = make_baseline(baseline_groups, "x86_64", 8, "1.75", "abc123", 42);

    let mut current = BTreeMap::new();
    current.insert(
        "census/parse".to_string(),
        group_measurement(Some(1000.0), 1.5),
    );

    let result = check_throughput(&baseline, &current, 0.05);
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("regression or missing data"),
        "NaN baseline throughput should fail the comparison"
    );
}
