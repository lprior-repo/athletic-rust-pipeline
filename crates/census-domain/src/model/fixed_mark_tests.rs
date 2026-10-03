use crate::model::{CentiMetres, CentiPoints, CentiSeconds};

#[test]
fn fixed_point_display_preserves_sign_and_integer_extremes(
) -> Result<(), Box<dyn std::error::Error>> {
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
        check!(eq; CentiSeconds::new(value).to_string(), expected);
        check!(eq; CentiMetres::new(value).to_string(), expected);
        check!(eq; CentiPoints::new(value).to_string(), expected);
    }
    Ok(())
}

#[test]
fn time_order_is_integer_order() -> Result<(), Box<dyn std::error::Error>> {
    check!(CentiSeconds::new(1094) < CentiSeconds::new(1102));
    check!(CentiSeconds::new(1094) < CentiSeconds::new(1230));
    check!(CentiSeconds::new(58123) < CentiSeconds::new(59511));
    Ok(())
}

#[test]
fn distance_order_is_integer_order() -> Result<(), Box<dyn std::error::Error>> {
    check!(CentiMetres::new(610) < CentiMetres::new(642));
    check!(CentiMetres::new(1868) < CentiMetres::new(1880));
    Ok(())
}

#[test]
fn points_order_is_integer_order() -> Result<(), Box<dyn std::error::Error>> {
    check!(CentiPoints::new(290000) < CentiPoints::new(312000));
    check!(CentiPoints::new(845600) < CentiPoints::new(915800));
    Ok(())
}

#[test]
fn mark_equality_is_exact() -> Result<(), Box<dyn std::error::Error>> {
    let a = CentiSeconds::new(1094);
    let b = CentiSeconds::new(1094);
    check!(eq; a, b);
    Ok(())
}

#[test]
fn centi_seconds_round_trips() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        (10.94, "10.94"),
        (59.99, "59.99"),
        (60.00, "60.00"),
        (114.32, "114.32"),
        (281.23, "281.23"),
        (932.1, "932.10"),
    ];
    for (input, _expected) in cases {
        let cs = CentiSeconds::try_from_seconds_f64(input).ok_or("fixture is in range")?;
        let back = cs.as_seconds_f64();
        check!(
            (back - input).abs() < 0.005,
            "{input}→{back} (expected ~{input})"
        );
    }
    Ok(())
}

#[test]
fn centi_metres_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [
        (6.42, "6.42"),
        (12.34, "12.34"),
        (18.68, "18.68"),
        (2.02, "2.02"),
    ];
    for (input, _expected) in cases {
        let cm = CentiMetres::try_from_metres_f64(input).ok_or("fixture is in range")?;
        let back = cm.as_metres_f64();
        check!(
            (back - input).abs() < 0.005,
            "{input}→{back} (expected ~{input})"
        );
    }
    Ok(())
}

#[test]
fn centi_points_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let cases = [3120.0, 2900.0, 8456.0, 9158.0];
    for input in cases {
        let cp = CentiPoints::try_from_points_f64(input).ok_or("fixture is in range")?;
        let back = cp.as_points_f64();
        check!((back - input).abs() < 0.005, "{input}→{back}");
    }
    Ok(())
}

#[test]
fn float_comparison_fails_where_fixed_point_succeeds() -> Result<(), Box<dyn std::error::Error>> {
    let left: f64 = 0.1 + 0.2 + 0.3;
    let right: f64 = 0.1 + (0.2 + 0.3);
    check!(ne; left.to_bits(),
    right.to_bits(),
    "f64 associativity diverges");

    let cs_left = CentiSeconds::try_from_seconds_f64(left).ok_or("finite sum")?;
    let cs_right = CentiSeconds::try_from_seconds_f64(right).ok_or("finite sum")?;
    check!(eq; cs_left, cs_right, "fixed-point unifies the two paths");
    Ok(())
}

#[test]
fn field_mark_comparison_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
    let metres_f64: f64 = (61.0 * 12.0 + 3.50) * 0.0254;
    let cm = CentiMetres::try_from_metres_f64(metres_f64).ok_or("61-03.50 is in range")?;

    check!(eq; cm.value(), 1868);
    check!(eq; cm,
    CentiMetres::new(1868),
    "same published precision → same cm integer");
    Ok(())
}

#[test]
fn legacy_float_time_deserialises() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"50.21"#;
    let cs: CentiSeconds = serde_json::from_str(json)?;
    check!(eq; cs, CentiSeconds::new(5021));
    Ok(())
}

#[test]
fn legacy_float_distance_deserialises() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"4.0703499999999995"#;
    let cm: CentiMetres = serde_json::from_str(json)?;
    check!(eq; cm, CentiMetres::new(407));
    Ok(())
}

#[test]
fn legacy_float_points_deserialises() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"8421.0"#;
    let cp: CentiPoints = serde_json::from_str(json)?;
    check!(eq; cp, CentiPoints::new(842100));
    Ok(())
}

#[test]
fn current_integer_time_deserialises() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"5021"#;
    let cs: CentiSeconds = serde_json::from_str(json)?;
    check!(eq; cs, CentiSeconds::new(5021));
    Ok(())
}

#[test]
fn current_integer_distance_deserialises() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"407"#;
    let cm: CentiMetres = serde_json::from_str(json)?;
    check!(eq; cm, CentiMetres::new(407));
    Ok(())
}

#[test]
fn current_integer_points_deserialises() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"842100"#;
    let cp: CentiPoints = serde_json::from_str(json)?;
    check!(eq; cp, CentiPoints::new(842100));
    Ok(())
}

#[test]
fn legacy_float_overflow_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"30000000.0"#;
    let result: Result<CentiSeconds, _> = serde_json::from_str(json);
    check!(result.is_err(), "legacy float 30000000.0 should overflow");
    let err = result
        .err()
        .ok_or("expected fixed-mark overflow rejection")?
        .to_string();
    check!(
        err.contains("overflow") || err.contains("exceeds"),
        "error should mention overflow: {err}"
    );
    Ok(())
}

#[test]
fn scaling_refuses_what_does_not_fit() -> Result<(), Box<dyn std::error::Error>> {
    check!(eq; super::checked_hundredths(10.94), Some(1094));
    check!(eq; super::checked_hundredths(-10.94), Some(-1094));
    check!(eq; super::checked_hundredths(0.0), Some(0));
    check!(eq; super::checked_hundredths(f64::NAN), None);
    check!(eq; super::checked_hundredths(f64::INFINITY), None);
    check!(eq; super::checked_hundredths(f64::NEG_INFINITY), None);
    check!(eq; super::checked_hundredths(30_000_000.0), None);
    check!(eq; super::checked_hundredths(-30_000_000.0), None);
    Ok(())
}

#[test]
fn construction_refuses_what_cannot_be_stored() -> Result<(), Box<dyn std::error::Error>> {
    check!(eq; CentiSeconds::try_from_seconds_f64(f64::NAN), None);
    check!(eq; CentiSeconds::try_from_seconds_f64(f64::INFINITY), None);
    check!(eq; CentiSeconds::try_from_seconds_f64(f64::NEG_INFINITY), None);
    check!(eq; CentiSeconds::try_from_seconds_f64(30_000_000.0), None);
    check!(eq; CentiSeconds::try_from_seconds_f64(-30_000_000.0), None);
    check!(eq; CentiMetres::try_from_metres_f64(f64::NAN), None);
    check!(eq; CentiMetres::try_from_metres_f64(f64::INFINITY), None);
    check!(eq; CentiPoints::try_from_points_f64(f64::NAN), None);
    check!(eq; CentiPoints::try_from_points_f64(f64::INFINITY), None);

    check!(eq; CentiSeconds::try_from_seconds_f64(10.94),
    Some(CentiSeconds::new(1094)));
    check!(eq; CentiSeconds::try_from_seconds_f64(-10.94),
    Some(CentiSeconds::new(-1094)));
    Ok(())
}

#[test]
fn integer_wire_form_is_the_stored_sub_unit() -> Result<(), Box<dyn std::error::Error>> {
    let cs: CentiSeconds = serde_json::from_str("60")?;
    check!(eq; cs.value(), 60);
    Ok(())
}

#[test]
fn float_wire_form_is_whole_units_scaled_by_one_hundred() -> Result<(), Box<dyn std::error::Error>>
{
    let cs: CentiSeconds = serde_json::from_str("60.0")?;
    check!(eq; cs.value(), 6000);

    let cm: CentiMetres = serde_json::from_str("4.07")?;
    check!(eq; cm.value(), 407);

    let cp: CentiPoints = serde_json::from_str("3456.0")?;
    check!(eq; cp.value(), 345_600);
    Ok(())
}

#[test]
fn non_finite_floats_are_refused_by_the_reader_not_saturated(
) -> Result<(), Box<dyn std::error::Error>> {
    use serde::de::value::{Error, F64Deserializer};
    use serde::de::IntoDeserializer;
    use serde::Deserialize;

    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let input: F64Deserializer<Error> = value.into_deserializer();
        check!(
            CentiSeconds::deserialize(input).is_err(),
            "{value} must be refused by the mark reader, not saturated to a bound"
        );
    }
    Ok(())
}

#[test]
fn u32_past_i32_max_is_refused_not_wrapped() -> Result<(), Box<dyn std::error::Error>> {
    use serde::de::value::{Error, U32Deserializer};
    use serde::de::IntoDeserializer;
    use serde::Deserialize;

    let over: U32Deserializer<Error> = 3_000_000_000u32.into_deserializer();
    check!(
        CentiSeconds::deserialize(over).is_err(),
        "3 000 000 000 centiseconds must be refused, not wrapped to -1 294 967 296"
    );

    let fits: U32Deserializer<Error> = 1094u32.into_deserializer();
    check!(eq; CentiSeconds::deserialize(fits).ok(),
    Some(CentiSeconds::new(1094)));
    Ok(())
}

#[test]
fn serialise_time_emits_raw_integer() -> Result<(), Box<dyn std::error::Error>> {
    let cs = CentiSeconds::new(5021);
    let json = serde_json::to_string(&cs)?;
    check!(eq; json, "5021");
    Ok(())
}

#[test]
fn serialise_distance_emits_raw_integer() -> Result<(), Box<dyn std::error::Error>> {
    let cm = CentiMetres::new(407);
    let json = serde_json::to_string(&cm)?;
    check!(eq; json, "407");
    Ok(())
}

#[test]
fn serialise_points_emits_raw_integer() -> Result<(), Box<dyn std::error::Error>> {
    let cp = CentiPoints::new(842100);
    let json = serde_json::to_string(&cp)?;
    check!(eq; json, "842100");
    Ok(())
}

use crate::model::Mark;

#[test]
fn mark_time_seconds_round_trips_legacy_json() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{"TimeSeconds":50.21}"#;
    let mark: Mark = serde_json::from_str(json)?;
    check!(eq; mark, Mark::TimeSeconds(CentiSeconds::new(5021)));
    Ok(())
}

#[test]
fn mark_distance_metres_round_trips_legacy_json() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{"DistanceMetres":4.0703499999999995}"#;
    let mark: Mark = serde_json::from_str(json)?;
    check!(eq; mark, Mark::DistanceMetres(CentiMetres::new(407)));
    Ok(())
}

#[test]
fn mark_field_imperial_round_trips_legacy_json() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{"FieldImperial":{"feet_mark":"5' 4\"","metres":162.56}}"#;
    let mark: Mark = serde_json::from_str(json)?;
    check!(eq; mark,
    Mark::FieldImperial {
        feet_mark: "5' 4\"".into(),
        metres: CentiMetres::new(16256),
    });
    Ok(())
}

#[test]
fn mark_points_round_trips_legacy_json() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"{"Points":8421.0}"#;
    let mark: Mark = serde_json::from_str(json)?;
    check!(eq; mark, Mark::Points(CentiPoints::new(842100)));
    Ok(())
}

#[test]
fn string_token_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#""10.94""#;
    let result: Result<CentiSeconds, _> = serde_json::from_str(json);
    check!(result.is_err(), "string token should be rejected");
    Ok(())
}

#[test]
fn null_token_is_error() -> Result<(), Box<dyn std::error::Error>> {
    let json = r#"null"#;
    let result: Result<CentiSeconds, _> = serde_json::from_str(json);
    check!(result.is_err(), "null should be rejected");
    Ok(())
}

#[test]
fn decimal_conversion_preserves_legacy_rounding_and_storage_bounds(
) -> Result<(), Box<dyn std::error::Error>> {
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
        check!(eq; CentiSeconds::try_from_seconds_f64(value).map(CentiSeconds::value),
        expected);
        check!(eq; CentiMetres::try_from_metres_f64(value).map(CentiMetres::value),
        expected);
        check!(eq; CentiPoints::try_from_points_f64(value).map(CentiPoints::value),
        expected);
    }
    Ok(())
}
