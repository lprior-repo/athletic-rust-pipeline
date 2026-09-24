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
// ── Serde: legacy float → integer deserialisation ───────────────────────────────

#[test]
fn legacy_float_time_deserialises() {
    let json = r#"50.21"#;
    let cs: CentiSeconds = serde_json::from_str(json).unwrap();
    assert_eq!(cs, CentiSeconds(5021));
}

#[test]
fn legacy_float_distance_deserialises() {
    let json = r#"4.0703499999999995"#;
    let cm: CentiMetres = serde_json::from_str(json).unwrap();
    assert_eq!(cm, CentiMetres(407));
}

#[test]
fn legacy_float_points_deserialises() {
    let json = r#"8421.0"#;
    let cp: CentiPoints = serde_json::from_str(json).unwrap();
    assert_eq!(cp, CentiPoints(842100));
}

// ── Serde: current integer → integer deserialisation ────────────────────────────

#[test]
fn current_integer_time_deserialises() {
    let json = r#"5021"#;
    let cs: CentiSeconds = serde_json::from_str(json).unwrap();
    assert_eq!(cs, CentiSeconds(5021));
}

#[test]
fn current_integer_distance_deserialises() {
    let json = r#"407"#;
    let cm: CentiMetres = serde_json::from_str(json).unwrap();
    assert_eq!(cm, CentiMetres(407));
}

#[test]
fn current_integer_points_deserialises() {
    let json = r#"842100"#;
    let cp: CentiPoints = serde_json::from_str(json).unwrap();
    assert_eq!(cp, CentiPoints(842100));
}

// ── Serde: overflow on legacy float is a hard error ────────────────────────────

#[test]
fn legacy_float_overflow_is_error() {
    let json = r#"30000000.0"#; // ×100 = 3 000 000 000 > i32::MAX
    let result: Result<CentiSeconds, _> = serde_json::from_str(json);
    assert!(result.is_err(), "legacy float 30000000.0 should overflow");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("overflow") || err.contains("exceeds"),
        "error should mention overflow: {err}"
    );
}

// ── Scaling: the shared conversion's boundary behaviour ────────────────────────

#[test]
fn scaling_refuses_what_does_not_fit() {
    assert_eq!(super::checked_hundredths(10.94), Some(1094));
    assert_eq!(super::checked_hundredths(-10.94), Some(-1094));
    assert_eq!(super::checked_hundredths(0.0), Some(0));
    assert_eq!(super::checked_hundredths(f64::NAN), None);
    assert_eq!(super::checked_hundredths(f64::INFINITY), None);
    assert_eq!(super::checked_hundredths(f64::NEG_INFINITY), None);
    assert_eq!(super::checked_hundredths(30_000_000.0), None); // ×100 exceeds i32::MAX
    assert_eq!(super::checked_hundredths(-30_000_000.0), None);
}

#[test]
fn scaling_for_parsers_keeps_the_replaced_cast_semantics() {
    assert_eq!(super::hundredths(10.94), 1094);
    // The `as` this replaced read NaN as zero and pinned an overflow to the bound it ran past;
    // the parsers already range-check, so these are the unreachable cases it has to keep agreeing on.
    assert_eq!(super::hundredths(f64::NAN), 0);
    assert_eq!(super::hundredths(f64::INFINITY), i32::MAX);
    assert_eq!(super::hundredths(f64::NEG_INFINITY), i32::MIN);
    assert_eq!(super::hundredths(30_000_000.0), i32::MAX);
    assert_eq!(super::hundredths(-30_000_000.0), i32::MIN);
}

#[test]
fn u32_past_i32_max_is_refused_not_wrapped() {
    use serde::de::value::{Error, U32Deserializer};
    use serde::de::IntoDeserializer;
    use serde::Deserialize;

    let over: U32Deserializer<Error> = 3_000_000_000u32.into_deserializer();
    assert!(
        CentiSeconds::deserialize(over).is_err(),
        "3 000 000 000 centiseconds must be refused, not wrapped to -1 294 967 296"
    );

    let fits: U32Deserializer<Error> = 1094u32.into_deserializer();
    assert_eq!(
        CentiSeconds::deserialize(fits).ok(),
        Some(CentiSeconds(1094))
    );
}

// ── Serde: serialisation pins the raw integer form ─────────────────────────────

#[test]
fn serialise_time_emits_raw_integer() {
    let cs = CentiSeconds(5021);
    let json = serde_json::to_string(&cs).unwrap();
    assert_eq!(json, "5021");
}

#[test]
fn serialise_distance_emits_raw_integer() {
    let cm = CentiMetres(407);
    let json = serde_json::to_string(&cm).unwrap();
    assert_eq!(json, "407");
}

#[test]
fn serialise_points_emits_raw_integer() {
    let cp = CentiPoints(842100);
    let json = serde_json::to_string(&cp).unwrap();
    assert_eq!(json, "842100");
}

// ── Serde: whole Mark round-trip from legacy shape ─────────────────────────────

use crate::model::Mark;

#[test]
fn mark_time_seconds_round_trips_legacy_json() {
    let json = r#"{"TimeSeconds":50.21}"#;
    let mark: Mark = serde_json::from_str(json).unwrap();
    assert_eq!(mark, Mark::TimeSeconds(CentiSeconds(5021)));
}

#[test]
fn mark_distance_metres_round_trips_legacy_json() {
    let json = r#"{"DistanceMetres":4.0703499999999995}"#;
    let mark: Mark = serde_json::from_str(json).unwrap();
    assert_eq!(mark, Mark::DistanceMetres(CentiMetres(407)));
}

#[test]
fn mark_field_imperial_round_trips_legacy_json() {
    let json = r#"{"FieldImperial":{"feet_mark":"5' 4\"","metres":162.56}}"#;
    let mark: Mark = serde_json::from_str(json).unwrap();
    assert_eq!(
        mark,
        Mark::FieldImperial {
            feet_mark: "5' 4\"".into(),
            metres: CentiMetres(16256),
        }
    );
}

#[test]
fn mark_points_round_trips_legacy_json() {
    let json = r#"{"Points":8421.0}"#;
    let mark: Mark = serde_json::from_str(json).unwrap();
    assert_eq!(mark, Mark::Points(CentiPoints(842100)));
}

// ── Serde: wrong token types are errors ────────────────────────────────────────

#[test]
fn string_token_is_error() {
    let json = r#""10.94""#;
    let result: Result<CentiSeconds, _> = serde_json::from_str(json);
    assert!(result.is_err(), "string token should be rejected");
}

#[test]
fn null_token_is_error() {
    let json = r#"null"#;
    let result: Result<CentiSeconds, _> = serde_json::from_str(json);
    assert!(result.is_err(), "null should be rejected");
}
