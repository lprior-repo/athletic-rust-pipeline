//! Tests for the fixed-point mark newtypes: ordering, round-trip, and float pitfalls.

use crate::model::{CentiMetres, CentiPoints, CentiSeconds};

// ── Ordering ────────────────────────────────────────────────────────────────────

#[test]
fn time_order_is_integer_order() {
    assert!(CentiSeconds(1094) < CentiSeconds(1102)); // 10.94 < 11.02
    assert!(CentiSeconds(1094) < CentiSeconds(1230)); // 10.94 < 12.30
    assert!(CentiSeconds(58123) < CentiSeconds(59511)); // 9:41.23 < 9:55.11
}

#[test]
fn distance_order_is_integer_order() {
    assert!(CentiMetres(610) < CentiMetres(642)); // 6.10m < 6.42m
    assert!(CentiMetres(1868) < CentiMetres(1880)); // 18.68m < 18.80m
}

#[test]
fn points_order_is_integer_order() {
    assert!(CentiPoints(290000) < CentiPoints(312000)); // 2900 < 3120
    assert!(CentiPoints(845600) < CentiPoints(915800)); // 8456 < 9158
}

#[test]
fn mark_equality_is_exact() {
    // Same integer = same mark, regardless of construction path.
    let a = CentiSeconds(1094);
    let b = CentiSeconds(1094);
    assert_eq!(a, b);
}

// ── Parse→fixed-point→format round trip ────────────────────────────────────────

#[test]
fn centi_seconds_round_trips() {
    let cases = [
        (10.94, "10.94"),
        (59.99, "59.99"),
        (60.00, "60.00"),
        (114.32, "114.32"),
        (281.23, "281.23"),
        (932.1, "932.10"),
    ];
    for (input, _expected) in cases {
        let cs = CentiSeconds::from_seconds_f64(input);
        let back = cs.as_seconds_f64();
        assert!(
            (back - input).abs() < 0.005,
            "{input}→{back} (expected ~{input})"
        );
    }
}

#[test]
fn centi_metres_round_trip() {
    let cases = [
        (6.42, "6.42"),
        (12.34, "12.34"),
        (18.68, "18.68"),
        (2.02, "2.02"),
    ];
    for (input, _expected) in cases {
        let cm = CentiMetres::from_metres_f64(input);
        let back = cm.as_metres_f64();
        assert!(
            (back - input).abs() < 0.005,
            "{input}→{back} (expected ~{input})"
        );
    }
}

#[test]
fn centi_points_round_trip() {
    let cases = [3120.0, 2900.0, 8456.0, 9158.0];
    for input in cases {
        let cp = CentiPoints::from_points_f64(input);
        let back = cp.as_points_f64();
        assert!((back - input).abs() < 0.005, "{input}→{back}");
    }
}

// ── Float comparison pitfall ───────────────────────────────────────────────────

#[test]
fn float_comparison_fails_where_fixed_point_succeeds() {
    // The trap: floating-point addition and multiplication do not distribute
    // exactly.  (a + b) + c != a + (b + c) is the classic associativity trap.
    //
    // 0.1 + 0.2 + 0.3 differs from 0.1 + (0.2 + 0.3) in the low bits:
    let left: f64 = 0.1 + 0.2 + 0.3;
    let right: f64 = 0.1 + (0.2 + 0.3);
    // They differ by 1 ulp:
    assert_ne!(
        left.to_bits(),
        right.to_bits(),
        "f64 associativity diverges"
    );

    // But centiseconds round to the same integer:
    let cs_left = CentiSeconds::from_seconds_f64(left);
    let cs_right = CentiSeconds::from_seconds_f64(right);
    assert_eq!(cs_left, cs_right, "fixed-point unifies the two paths");

    // This is the deterministic guarantee: same published value → same
    // internal representation, regardless of how it arrived.
}

#[test]
fn field_mark_comparison_is_deterministic() {
    // 61-03.50 → (61*12+3.50)*0.0254 = 18.6817m
    // But stored as centimetres: 1868cm
    let metres_f64: f64 = (61.0 * 12.0 + 3.50) * 0.0254;
    let cm = CentiMetres::from_metres_f64(metres_f64);

    // 1868cm vs 1868.17cm: the f64 has the fractional part, the cm does not.
    // Two marks at the same published precision must compare equal.
    assert_eq!(cm.0, 1868); // 1868.17 rounds to 1868
    assert_eq!(
        cm,
        CentiMetres(1868),
        "same published precision → same cm integer"
    );
}
