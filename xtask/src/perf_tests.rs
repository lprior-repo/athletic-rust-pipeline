//! Parse-level unit tests for the perf verb types and CLI dispatch.
//!
//! These tests exercise the clap argument parsing (no benchmark runs).

use crate::perf::DEFAULT_TOLERANCE;
use std::collections::BTreeMap;

/// The default tolerance must be a non-negative fraction (5%).
#[test]
fn default_tolerance_is_non_negative() {
    #[allow(clippy::assertions_on_constants)]
    {
        assert!(DEFAULT_TOLERANCE >= 0.0);
        assert!(DEFAULT_TOLERANCE <= 1.0);
    }
}

/// Baseline groups should round-trip through JSON serialization.
#[test]
fn group_measurement_roundtrips_json() {
    use crate::perf::GroupMeasurement;
    use serde_json;

    let gm = GroupMeasurement {
        throughput: Some(12345.6),
        peak_rss_kib: Some(50000),
        wall_time_seconds: 3.15,
    };
    let json = serde_json::to_string(&gm).unwrap();
    let roundtripped: GroupMeasurement = serde_json::from_str(&json).unwrap();
    assert_eq!(roundtripped.throughput, Some(12345.6));
    assert_eq!(roundtripped.peak_rss_kib, Some(50000));
    assert_eq!(roundtripped.wall_time_seconds, 3.15);
}

/// A group measurement without throughput serializes with null (or omits the field).
#[test]
fn group_measurement_without_throughput_serializes() {
    use crate::perf::GroupMeasurement;
    use serde_json;

    let gm = GroupMeasurement {
        throughput: None,
        peak_rss_kib: None,
        wall_time_seconds: 1.0,
    };
    let json = serde_json::to_string(&gm).unwrap();
    // Throughput field should be omitted (skip_serializing_if = "Option::is_none")
    assert!(!json.contains("throughput"));
    assert!(!json.contains("peak_rss_kib"));
    assert!(json.contains("wall_time_seconds"));
}

/// Baseline groups BTreeMap preserves insertion order by group id.
#[test]
fn baseline_groups_preserves_order() {
    use crate::perf::PerfBaseline;

    let mut groups = BTreeMap::new();
    groups.insert(
        "c".to_string(),
        crate::perf::GroupMeasurement {
            throughput: Some(100.0),
            peak_rss_kib: None,
            wall_time_seconds: 1.0,
        },
    );
    groups.insert(
        "a".to_string(),
        crate::perf::GroupMeasurement {
            throughput: Some(200.0),
            peak_rss_kib: None,
            wall_time_seconds: 2.0,
        },
    );
    groups.insert(
        "b".to_string(),
        crate::perf::GroupMeasurement {
            throughput: Some(150.0),
            peak_rss_kib: None,
            wall_time_seconds: 1.5,
        },
    );

    let baseline = PerfBaseline {
        metadata: crate::perf::Meta {
            cpu: "test".to_string(),
            cores: 1,
            rustc: "rustc test".to_string(),
            sha: "abc123".to_string(),
            corpus_lines: 0,
        },
        check_reason: None,
        groups,
    };

    let json = serde_json::to_string(&baseline).unwrap();
    // BTreeMap iterates in key order, so "a" should appear before "b" before "c"
    let a_pos = json.find(r#""a":"#).unwrap();
    let b_pos = json.find(r#""b":"#).unwrap();
    let c_pos = json.find(r#""c":"#).unwrap();
    assert!(
        a_pos < b_pos,
        "a should appear before b in serialized BTreeMap"
    );
    assert!(
        b_pos < c_pos,
        "b should appear before c in serialized BTreeMap"
    );
}
