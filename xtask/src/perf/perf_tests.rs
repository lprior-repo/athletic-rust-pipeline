use super::compare::check_throughput;
use super::{GroupMeasurement, Meta, PerfBaseline, Throughput};
use std::collections::BTreeMap;

fn measurement(throughput: Option<f64>, seconds: f64) -> GroupMeasurement {
    GroupMeasurement {
        throughput: throughput.map(Throughput::Elements),
        peak_rss_kib: None,
        wall_time_seconds: seconds,
    }
}

fn baseline(measurement: GroupMeasurement) -> PerfBaseline {
    PerfBaseline {
        metadata: Meta {
            cpu: "test".into(),
            cores: 1,
            rustc: "test".into(),
            sha: "test".into(),
            corpus_lines: 1,
        },
        check_reason: None,
        groups: BTreeMap::from([("group/benchmark".into(), measurement)]),
    }
}

#[test]
fn throughput_decline_crosses_the_tolerance_boundary() {
    let baseline = baseline(measurement(Some(100.0), 1.0));
    for (rate, accepted) in [(110.0, true), (100.0, true), (95.0, true), (94.9, false)] {
        let current = BTreeMap::from([("group/benchmark".into(), measurement(Some(rate), 1.0))]);
        assert_eq!(
            check_throughput(&baseline, &current, 0.05).is_ok(),
            accepted,
            "{rate}"
        );
    }
}

#[test]
fn timing_only_benchmarks_detect_regression_not_improvement() {
    let baseline = baseline(measurement(None, 1.0));
    for (seconds, accepted) in [
        (0.5, true),
        (1.0, true),
        (1.04, true),
        (1.05, true),
        (1.06, false),
    ] {
        let current = BTreeMap::from([("group/benchmark".into(), measurement(None, seconds))]);
        assert_eq!(
            check_throughput(&baseline, &current, 0.05).is_ok(),
            accepted,
            "{seconds}"
        );
    }
}

#[test]
fn changing_the_measurement_kind_is_rejected_in_both_directions() {
    for (old, new) in [(None, Some(1.0)), (Some(1.0), None)] {
        let baseline = baseline(measurement(old, 1.0));
        let current = BTreeMap::from([("group/benchmark".into(), measurement(new, 1.0))]);
        assert!(check_throughput(&baseline, &current, 0.05).is_err());
    }
}

#[test]
fn missing_extra_and_empty_benchmark_sets_are_rejected() {
    let baseline = baseline(measurement(Some(1.0), 1.0));
    let mut current = baseline.groups.clone();
    current.insert("other/benchmark".into(), measurement(Some(1.0), 1.0));
    assert!(check_throughput(&baseline, &current, 0.05).is_err());
    current.remove("group/benchmark");
    assert!(check_throughput(&baseline, &current, 0.05).is_err());
    assert!(check_throughput(&baseline, &BTreeMap::new(), 0.05).is_err());
}

#[test]
fn invalid_metrics_are_rejected_on_either_side() {
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -1.0] {
        for bad in [
            measurement(Some(invalid), 1.0),
            measurement(Some(1.0), invalid),
        ] {
            let good = measurement(Some(1.0), 1.0);
            let baseline = baseline(bad.clone());
            let current = BTreeMap::from([("group/benchmark".into(), good.clone())]);
            assert!(check_throughput(&baseline, &current, 0.05).is_err());
            let baseline = super::tests::baseline(good);
            let current = BTreeMap::from([("group/benchmark".into(), bad)]);
            assert!(check_throughput(&baseline, &current, 0.05).is_err());
        }
    }
}

#[test]
fn tolerance_cannot_disable_the_regression_gate() {
    let baseline = baseline(measurement(Some(1.0), 1.0));
    for tolerance in [f64::NAN, f64::INFINITY, -0.1, 1.0, 2.0] {
        assert!(check_throughput(&baseline, &baseline.groups, tolerance).is_err());
    }
}
