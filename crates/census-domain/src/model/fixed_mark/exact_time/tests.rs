use super::{ExactSeconds, TimeError};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn clock_components_use_exact_decimal_nanoseconds() -> TestResult {
    for (raw, nanoseconds, precision, displayed) in [
        ("1:00", 60_000_000_000, 0, "60"),
        ("0:00.000000001", 1, 9, "0.000000001"),
        ("1:02.345", 62_345_000_000, 3, "62.345"),
        ("1:02:03.450000001", 3_723_450_000_001, 9, "3723.450000001"),
        (
            "153722867:16.854775807",
            i64::MAX,
            9,
            "9223372036.854775807",
        ),
    ] {
        let time = ExactSeconds::parse_clock(raw)?;
        check!(eq; (time.value(), time.precision(), time.to_string()), (nanoseconds, precision, displayed.to_string()));
    }
    Ok(())
}

#[test]
fn invalid_clock_components_and_overflow_are_distinct_refusals() -> TestResult {
    for raw in [
        "1:60",
        "1:00:60",
        "1:60:00",
        "1:2:3:4",
        "0:00",
        "1.5:00",
        "-1:00",
        "1:NaN",
        "1:inf",
        "1:00.0000000001",
    ] {
        check!(eq; ExactSeconds::parse_clock(raw), Err(TimeError::Invalid), "{raw}");
    }
    for raw in [
        "153722867:16.854775808",
        "153722868:00",
        "9999999999999999999:00",
    ] {
        check!(eq; ExactSeconds::parse_clock(raw), Err(TimeError::Overflow), "{raw}");
    }
    Ok(())
}

#[test]
fn display_only_seconds_conversion_does_not_fabricate_zero_at_the_upper_boundary() -> TestResult {
    let maximum = ExactSeconds::parse("9223372036.854775807")?;
    check!(eq; maximum.try_as_seconds_f64(), Some(9_223_372_036.854_776));
    let minimum = ExactSeconds::parse("0.000000001")?;
    check!(eq; minimum.try_as_seconds_f64(), Some(0.000_000_001));
    Ok(())
}
