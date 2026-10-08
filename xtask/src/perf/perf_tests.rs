use super::compare::check_throughput;
use super::{GroupMeasurement, Meta, PerfBaseline, Throughput};
use std::collections::BTreeMap;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn measurement(throughput: Option<f64>, seconds: f64) -> GroupMeasurement {
    GroupMeasurement {
        throughput: throughput.map(Throughput::Elements),
        peak_rss_kib: Some(100),
        allocation_count: Some(100),
        allocated_bytes: Some(100),
        tail_time_seconds: Some(1.0),
        timing_scope: None,
        wall_time_seconds: seconds,
    }
}

pub(super) fn capture_measurement() -> GroupMeasurement {
    let mut measured = measurement(Some(196.0), 1.0);
    measured.timing_scope = Some("capture-publication-independent-readback/v1".into());
    measured
}

fn groups(measured: GroupMeasurement) -> BTreeMap<String, GroupMeasurement> {
    BTreeMap::from([
        ("group/benchmark".into(), measured),
        (
            "pipeline/capture_export/captured_live_wiaa_co2027".into(),
            capture_measurement(),
        ),
    ])
}

fn baseline(measurement: GroupMeasurement) -> PerfBaseline {
    PerfBaseline {
        metadata: Meta {
            cpu: "test".into(),
            cores: 1,
            rustc: "test".into(),
            sha: "test".into(),
            corpus_lines: 1,
            corpus_sha256: "a".repeat(64),
        },
        check_reason: None,
        groups: groups(measurement),
    }
}

#[test]
fn throughput_decline_crosses_the_tolerance_boundary() -> TestResult {
    let baseline = baseline(measurement(Some(100.0), 1.0));
    for (rate, accepted) in [(110.0, true), (100.0, true), (95.0, true), (94.9, false)] {
        let current = groups(measurement(Some(rate), 1.0));
        check!(eq; check_throughput(&baseline, &current, 0.05).is_ok(), accepted, "{rate}");
    }
    Ok(())
}

#[test]
fn missing_or_null_rates_refuse_comparison_with_other_metrics_complete() -> TestResult {
    for raw in [
        r#"{"wall_time_seconds":1,"tail_time_seconds":1,"peak_rss_kib":100,"allocation_count":100,"allocated_bytes":100}"#,
        r#"{"throughput":null,"wall_time_seconds":1,"tail_time_seconds":1,"peak_rss_kib":100,"allocation_count":100,"allocated_bytes":100}"#,
    ] {
        let missing: GroupMeasurement = serde_json::from_str(raw)?;
        for (old, new) in [
            (missing.clone(), missing.clone()),
            (missing.clone(), measurement(Some(100.0), 1.0)),
            (measurement(Some(100.0), 1.0), missing.clone()),
        ] {
            let baseline = baseline(old);
            super::compare::compare_environment(&baseline.metadata, &baseline.metadata)?;
            let mut current = baseline.groups.clone();
            current.insert("group/benchmark".into(), new);
            let error = check_throughput(&baseline, &current, 0.05)
                .err()
                .ok_or("missing throughput was accepted")?;
            check!(format!("{error:#}").contains("throughput"));
        }
    }
    Ok(())
}

#[test]
fn invalid_rates_refuse_comparison_with_other_metrics_complete() -> TestResult {
    for rate in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for (old, new) in [
            (measurement(Some(rate), 1.0), measurement(Some(100.0), 1.0)),
            (measurement(Some(100.0), 1.0), measurement(Some(rate), 1.0)),
        ] {
            let baseline = baseline(old);
            super::compare::compare_environment(&baseline.metadata, &baseline.metadata)?;
            let mut current = baseline.groups.clone();
            current.insert("group/benchmark".into(), new);
            let error = check_throughput(&baseline, &current, 0.05)
                .err()
                .ok_or("invalid throughput was accepted")?;
            check!(format!("{error:#}").contains("throughput"));
        }
    }
    Ok(())
}

#[test]
fn omitting_capture_export_from_both_measurement_sets_refuses_comparison() -> TestResult {
    let mut baseline = baseline(measurement(Some(100.0), 1.0));
    baseline
        .groups
        .remove("pipeline/capture_export/captured_live_wiaa_co2027");
    super::compare::compare_environment(&baseline.metadata, &baseline.metadata)?;
    let error = check_throughput(&baseline, &baseline.groups, 0.05)
        .err()
        .ok_or("required capture_export workload was omitted")?;
    check!(format!("{error:#}").contains("pipeline/capture_export/captured_live_wiaa_co2027"));
    Ok(())
}

#[test]
fn omitting_capture_export_from_either_measurement_set_refuses_comparison() -> TestResult {
    let good = groups(measurement(Some(100.0), 1.0));
    let mut missing = good.clone();
    missing.remove("pipeline/capture_export/captured_live_wiaa_co2027");
    for (old, current) in [(missing.clone(), good.clone()), (good, missing)] {
        let mut baseline = baseline(measurement(Some(100.0), 1.0));
        baseline.groups = old;
        let error = check_throughput(&baseline, &current, 0.05)
            .err()
            .ok_or("required capture_export workload was omitted")?;
        check!(format!("{error:#}").contains("pipeline/capture_export/captured_live_wiaa_co2027"));
    }
    Ok(())
}

#[test]
fn compatible_fully_measured_capture_export_and_core_rates_pass() -> TestResult {
    let baseline = baseline(measurement(Some(100.0), 1.0));
    super::compare::compare_environment(&baseline.metadata, &baseline.metadata)?;
    check_throughput(&baseline, &baseline.groups, 0.05)?;
    Ok(())
}

#[test]
fn missing_extra_and_empty_benchmark_sets_are_rejected() -> TestResult {
    let baseline = baseline(measurement(Some(1.0), 1.0));
    let mut current = baseline.groups.clone();
    current.insert("other/benchmark".into(), measurement(Some(1.0), 1.0));
    check!(check_throughput(&baseline, &current, 0.05).is_err());
    current.remove("group/benchmark");
    check!(check_throughput(&baseline, &current, 0.05).is_err());
    check!(check_throughput(&baseline, &BTreeMap::new(), 0.05).is_err());
    Ok(())
}

#[test]
fn invalid_wall_time_is_rejected_on_either_side() -> TestResult {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -1.0] {
        for (old, new) in [
            (
                measurement(Some(100.0), invalid),
                measurement(Some(100.0), 1.0),
            ),
            (
                measurement(Some(100.0), 1.0),
                measurement(Some(100.0), invalid),
            ),
        ] {
            let baseline = baseline(old);
            let error = check_throughput(&baseline, &groups(new), 0.05)
                .err()
                .ok_or("invalid wall time was accepted")?;
            check!(format!("{error:#}").contains("wall time"));
        }
    }
    Ok(())
}

#[test]
fn tolerance_cannot_disable_the_regression_gate() -> TestResult {
    let baseline = baseline(measurement(Some(1.0), 1.0));
    for tolerance in [f64::NAN, f64::INFINITY, -0.1, 1.0, 2.0] {
        check!(check_throughput(&baseline, &baseline.groups, tolerance).is_err());
    }
    Ok(())
}

#[test]
fn absent_or_pre_readback_capture_timing_scope_refuses_comparison() -> TestResult {
    for scope in [None, Some("capture-publication/v1"), Some("")] {
        let mut incompatible = capture_measurement();
        incompatible.timing_scope = scope.map(str::to_owned);
        for (old, new) in [
            (incompatible.clone(), incompatible.clone()),
            (incompatible.clone(), capture_measurement()),
            (capture_measurement(), incompatible.clone()),
        ] {
            let mut baseline = baseline(measurement(Some(100.0), 1.0));
            baseline.groups.insert(
                "pipeline/capture_export/captured_live_wiaa_co2027".into(),
                old,
            );
            let baseline: PerfBaseline = serde_json::from_slice(&serde_json::to_vec(&baseline)?)?;
            let mut current = groups(measurement(Some(100.0), 1.0));
            current.insert(
                "pipeline/capture_export/captured_live_wiaa_co2027".into(),
                new,
            );
            let error = check_throughput(&baseline, &current, 0.05)
                .err()
                .ok_or("incompatible capture timing scope was accepted")?;
            check!(format!("{error:#}").contains("incompatible timing scope"));
        }
    }
    Ok(())
}

#[path = "memory_tests.rs"]
mod memory_tests;
