use crate::perf::bench::parse_bencher_line;

const BENCHER_SAMPLE: &str = "test census/parse/hynek_lines_from_html ... bench:   12,345 ns/iter (+/- 1,234)\ntest census/parse/hynek_parse ... bench:   23,456 ns/iter (+/- 2,345)\n";

const BENCHER_NO_THROUGHPUT: &str =
    "test census/school_index/normalize_label ... bench:    1,234 ns/iter (+/- 123)\n";

const GARBAGE_OUTPUT: &str =
    "this is not criterion output\nname=something\nmetric=wall_seconds value=1.0\n";

#[test]
fn parse_bencher_line_normal_ns() {
    let line = "test census/parse/hynek_lines_from_html ... bench:   12,345 ns/iter (+/- 1,234)";
    let (name, ns) = parse_bencher_line(line).expect("should parse");
    assert_eq!(name, "census/parse/hynek_lines_from_html");
    assert_eq!(ns, 12_345.0);
}

#[test]
fn parse_bencher_line_second_benchmark() {
    let line = "test census/parse/hynek_parse ... bench:   23,456 ns/iter (+/- 2,345)";
    let (name, ns) = parse_bencher_line(line).expect("should parse");
    assert_eq!(name, "census/parse/hynek_parse");
    assert_eq!(ns, 23_456.0);
}

#[test]
fn parse_bencher_line_microseconds() {
    for unit in &["μs", "µs"] {
        let line = format!("test group/fn ... bench: 123 {unit}/iter (+/- 10)");
        let (name, ns) = parse_bencher_line(&line).expect("should parse");
        assert_eq!(name, "group/fn");
        assert_eq!(ns, 123_000.0);
    }
}

#[test]
fn parse_bencher_line_milliseconds() {
    let line = "test group/fn ... bench: 5 ms/iter (+/- 1)";
    let (name, ns) = parse_bencher_line(line).expect("should parse");
    assert_eq!(name, "group/fn");
    assert_eq!(ns, 5_000_000.0);
}

#[test]
fn parse_bencher_line_seconds() {
    let line = "test slow_bench ... bench: 2.5 s/iter (+/- 0.1)";
    let (name, ns) = parse_bencher_line(line).expect("should parse");
    assert_eq!(name, "slow_bench");
    assert!((ns - 2_500_000_000.0).abs() < 1.0);
}

#[test]
fn parse_bencher_line_missing_bench() {
    let line = "test group/fn something wrong";
    let err = parse_bencher_line(line).expect_err("should error");
    assert!(err.to_string().contains("bench"));
}

#[test]
fn parse_bencher_line_empty() {
    let err = parse_bencher_line("").expect_err("should error");
    assert!(err.to_string().contains("name"));
}

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

#[test]
fn parse_bencher_no_throughput() {
    let line = BENCHER_NO_THROUGHPUT.trim();
    let (name, ns) = parse_bencher_line(line).expect("should parse");
    assert_eq!(name, "census/school_index/normalize_label");
    assert_eq!(ns, 1_234.0);
}

#[test]
fn parse_bencher_line_large_seconds() {
    let line = "test massive_group/massive_bench ... bench: 123.456 s/iter (+/- 12.345)";
    let (name, ns) = parse_bencher_line(line).expect("should parse");
    assert_eq!(name, "massive_group/massive_bench");
    assert!((ns - 123_456_000_000.0).abs() < 1.0);
}

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
