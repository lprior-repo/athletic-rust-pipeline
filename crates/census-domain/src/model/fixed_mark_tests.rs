//! Tests for the fixed-point mark newtypes: ordering, round-trip, and float pitfalls.

use crate::model::{CentiMetres, CentiPoints, CentiSeconds};

#[test]
fn fixed_point_display_preserves_sign_and_integer_extremes() {
    for (value, expected) in [
        (i32::MIN, "-21474836.48"),
        (-101, "-1.01"),
        (-100, "-1.00"),
        (-99, "-0.99"),
        (-5, "-0.05"),
        (-1, "-0.01"),
        (0, "0.00"),
        (1, "0.01"),
        (99, "0.99"),
        (100, "1.00"),
        (i32::MAX, "21474836.47"),
    ] {
        assert_eq!(CentiSeconds::new(value).to_string(), expected);
        assert_eq!(CentiMetres::new(value).to_string(), expected);
        assert_eq!(CentiPoints::new(value).to_string(), expected);
    }
}

#[test]
fn time_order_is_integer_order() {
    assert!(CentiSeconds::new(1094) < CentiSeconds::new(1102));
    assert!(CentiSeconds::new(1094) < CentiSeconds::new(1230));
    assert!(CentiSeconds::new(58123) < CentiSeconds::new(59511));
}

#[test]
fn distance_order_is_integer_order() {
    assert!(CentiMetres::new(610) < CentiMetres::new(642));
    assert!(CentiMetres::new(1868) < CentiMetres::new(1880));
}

#[test]
fn points_order_is_integer_order() {
    assert!(CentiPoints::new(290000) < CentiPoints::new(312000));
    assert!(CentiPoints::new(845600) < CentiPoints::new(915800));
}

#[test]
fn mark_equality_is_exact() {
    let a = CentiSeconds::new(1094);
    let b = CentiSeconds::new(1094);
    assert_eq!(a, b);
}

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
        let cs = CentiSeconds::try_from_seconds_f64(input).expect("fixture is in range");
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
        let cm = CentiMetres::try_from_metres_f64(input).expect("fixture is in range");
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
        let cp = CentiPoints::try_from_points_f64(input).expect("fixture is in range");
        let back = cp.as_points_f64();
        assert!((back - input).abs() < 0.005, "{input}→{back}");
    }
}

#[test]
fn float_comparison_fails_where_fixed_point_succeeds() {
    let left: f64 = 0.1 + 0.2 + 0.3;
    let right: f64 = 0.1 + (0.2 + 0.3);
    assert_ne!(
        left.to_bits(),
        right.to_bits(),
        "f64 associativity diverges"
    );

    let cs_left = CentiSeconds::try_from_seconds_f64(left).expect("finite sum");
    let cs_right = CentiSeconds::try_from_seconds_f64(right).expect("finite sum");
    assert_eq!(cs_left, cs_right, "fixed-point unifies the two paths");
}

#[test]
fn field_mark_comparison_is_deterministic() {
    let metres_f64: f64 = (61.0 * 12.0 + 3.50) * 0.0254;
    let cm = CentiMetres::try_from_metres_f64(metres_f64).expect("61-03.50 is in range");

    assert_eq!(cm.value(), 1868);
    assert_eq!(
        cm,
        CentiMetres::new(1868),
        "same published precision → same cm integer"
    );
}

#[test]
fn legacy_float_time_deserialises() {
    let json = r#"50.21"#;
    let cs: CentiSeconds = serde_json::from_str(json).unwrap();
    assert_eq!(cs, CentiSeconds::new(5021));
}

#[test]
fn legacy_float_distance_deserialises() {
    let json = r#"4.0703499999999995"#;
    let cm: CentiMetres = serde_json::from_str(json).unwrap();
    assert_eq!(cm, CentiMetres::new(407));
}

#[test]
fn legacy_float_points_deserialises() {
    let json = r#"8421.0"#;
    let cp: CentiPoints = serde_json::from_str(json).unwrap();
    assert_eq!(cp, CentiPoints::new(842100));
}

#[test]
fn current_integer_time_deserialises() {
    let json = r#"5021"#;
    let cs: CentiSeconds = serde_json::from_str(json).unwrap();
    assert_eq!(cs, CentiSeconds::new(5021));
}

#[test]
fn current_integer_distance_deserialises() {
    let json = r#"407"#;
    let cm: CentiMetres = serde_json::from_str(json).unwrap();
    assert_eq!(cm, CentiMetres::new(407));
}

#[test]
fn current_integer_points_deserialises() {
    let json = r#"842100"#;
    let cp: CentiPoints = serde_json::from_str(json).unwrap();
    assert_eq!(cp, CentiPoints::new(842100));
}

#[test]
fn legacy_float_overflow_is_error() {
    let json = r#"30000000.0"#;
    let result: Result<CentiSeconds, _> = serde_json::from_str(json);
    assert!(result.is_err(), "legacy float 30000000.0 should overflow");
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("overflow") || err.contains("exceeds"),
        "error should mention overflow: {err}"
    );
}

#[test]
fn scaling_refuses_what_does_not_fit() {
    assert_eq!(super::checked_hundredths(10.94), Some(1094));
    assert_eq!(super::checked_hundredths(-10.94), Some(-1094));
    assert_eq!(super::checked_hundredths(0.0), Some(0));
    assert_eq!(super::checked_hundredths(f64::NAN), None);
    assert_eq!(super::checked_hundredths(f64::INFINITY), None);
    assert_eq!(super::checked_hundredths(f64::NEG_INFINITY), None);
    assert_eq!(super::checked_hundredths(30_000_000.0), None);
    assert_eq!(super::checked_hundredths(-30_000_000.0), None);
}

#[test]
fn construction_refuses_what_cannot_be_stored() {
    assert_eq!(CentiSeconds::try_from_seconds_f64(f64::NAN), None);
    assert_eq!(CentiSeconds::try_from_seconds_f64(f64::INFINITY), None);
    assert_eq!(CentiSeconds::try_from_seconds_f64(f64::NEG_INFINITY), None);
    assert_eq!(CentiSeconds::try_from_seconds_f64(30_000_000.0), None);
    assert_eq!(CentiSeconds::try_from_seconds_f64(-30_000_000.0), None);
    assert_eq!(CentiMetres::try_from_metres_f64(f64::NAN), None);
    assert_eq!(CentiMetres::try_from_metres_f64(f64::INFINITY), None);
    assert_eq!(CentiPoints::try_from_points_f64(f64::NAN), None);
    assert_eq!(CentiPoints::try_from_points_f64(f64::INFINITY), None);

    assert_eq!(
        CentiSeconds::try_from_seconds_f64(10.94),
        Some(CentiSeconds::new(1094))
    );
    assert_eq!(
        CentiSeconds::try_from_seconds_f64(-10.94),
        Some(CentiSeconds::new(-1094))
    );
}

#[test]
fn integer_wire_form_is_the_stored_sub_unit() {
    let cs: CentiSeconds = serde_json::from_str("60").expect("integer wire form");
    assert_eq!(cs.value(), 60);
}

#[test]
fn float_wire_form_is_whole_units_scaled_by_one_hundred() {
    let cs: CentiSeconds = serde_json::from_str("60.0").expect("legacy float wire form");
    assert_eq!(cs.value(), 6000);

    let cm: CentiMetres = serde_json::from_str("4.07").expect("legacy float wire form");
    assert_eq!(cm.value(), 407);

    let cp: CentiPoints = serde_json::from_str("3456.0").expect("legacy float wire form");
    assert_eq!(cp.value(), 345_600);
}

#[test]
fn non_finite_floats_are_refused_by_the_reader_not_saturated() {
    use serde::de::value::{Error, F64Deserializer};
    use serde::de::IntoDeserializer;
    use serde::Deserialize;

    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let input: F64Deserializer<Error> = value.into_deserializer();
        assert!(
            CentiSeconds::deserialize(input).is_err(),
            "{value} must be refused by the mark reader, not saturated to a bound"
        );
    }
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
        Some(CentiSeconds::new(1094))
    );
}

#[test]
fn serialise_time_emits_raw_integer() {
    let cs = CentiSeconds::new(5021);
    let json = serde_json::to_string(&cs).unwrap();
    assert_eq!(json, "5021");
}

#[test]
fn serialise_distance_emits_raw_integer() {
    let cm = CentiMetres::new(407);
    let json = serde_json::to_string(&cm).unwrap();
    assert_eq!(json, "407");
}

#[test]
fn serialise_points_emits_raw_integer() {
    let cp = CentiPoints::new(842100);
    let json = serde_json::to_string(&cp).unwrap();
    assert_eq!(json, "842100");
}

use crate::model::Mark;

#[test]
fn mark_time_seconds_round_trips_legacy_json() {
    let json = r#"{"TimeSeconds":50.21}"#;
    let mark: Mark = serde_json::from_str(json).unwrap();
    assert_eq!(mark, Mark::TimeSeconds(CentiSeconds::new(5021)));
}

#[test]
fn mark_distance_metres_round_trips_legacy_json() {
    let json = r#"{"DistanceMetres":4.0703499999999995}"#;
    let mark: Mark = serde_json::from_str(json).unwrap();
    assert_eq!(mark, Mark::DistanceMetres(CentiMetres::new(407)));
}

#[test]
fn mark_field_imperial_round_trips_legacy_json() {
    let json = r#"{"FieldImperial":{"feet_mark":"5' 4\"","metres":162.56}}"#;
    let mark: Mark = serde_json::from_str(json).unwrap();
    assert_eq!(
        mark,
        Mark::FieldImperial {
            feet_mark: "5' 4\"".into(),
            metres: CentiMetres::new(16256),
        }
    );
}

#[test]
fn mark_points_round_trips_legacy_json() {
    let json = r#"{"Points":8421.0}"#;
    let mark: Mark = serde_json::from_str(json).unwrap();
    assert_eq!(mark, Mark::Points(CentiPoints::new(842100)));
}

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

#[test]
fn decimal_conversion_preserves_legacy_rounding_and_storage_bounds() {
    for (value, expected) in [
        (1.005, Some(100)),
        (-1.005, Some(-100)),
        (1.125, Some(113)),
        (-1.125, Some(-113)),
        (0.005, Some(1)),
        (-0.005, Some(-1)),
        (f64::MIN_POSITIVE, Some(0)),
        (f64::from(i32::MAX) / 100.0, Some(i32::MAX)),
        (f64::from(i32::MIN) / 100.0, Some(i32::MIN)),
        ((f64::from(i32::MAX) + 1.0) / 100.0, None),
        ((f64::from(i32::MIN) - 1.0) / 100.0, None),
        (f64::MAX, None),
        (f64::MIN, None),
    ] {
        assert_eq!(
            CentiSeconds::try_from_seconds_f64(value).map(CentiSeconds::value),
            expected
        );
        assert_eq!(
            CentiMetres::try_from_metres_f64(value).map(CentiMetres::value),
            expected
        );
        assert_eq!(
            CentiPoints::try_from_points_f64(value).map(CentiPoints::value),
            expected
        );
    }
}
