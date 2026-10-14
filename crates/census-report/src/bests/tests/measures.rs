use super::*;
use census_domain::model::performance_comparison::{field_micrometres, Measure};

#[test]
fn imperial_parser_rejects_malformed_unicode_and_fraction() {
    for input in ["5-3.1é", "5-3.中", "5-3.+1", "5-3.001"] {
        assert_eq!(field_micrometres(input), None, "{input}");
    }
    assert_eq!(field_micrometres("5-3.750"), Some(1_619_250));
}

#[test]
fn metric_scaling_widens_before_multiplication() {
    let mark = Mark::DistanceMetres(CentiMetres::new(i32::MAX));
    assert_eq!(
        Measure::Distance.value(&mark),
        Some(i64::from(i32::MAX) * 10_000)
    );
}

#[test]
fn measure_of_time() -> TestResult {
    let mark = Mark::TimeSeconds(ExactSeconds::parse("10.94")?);
    check!(
        eq;
        Measure::of(&mark),
        Some(Measure::Time)
    );
    Ok(())
}

#[test]
fn measure_of_distance() {
    let mark = Mark::DistanceMetres(CentiMetres::new(642));
    assert_eq!(Measure::of(&mark), Some(Measure::Distance));
}

#[test]
fn measure_of_field_imperial_is_distance() {
    let mark = Mark::FieldImperial {
        feet_mark: "20-00.00".to_string(),
        metres: CentiMetres::new(609),
    };
    assert_eq!(Measure::of(&mark), Some(Measure::Distance));
}

#[test]
fn measure_of_points() {
    let mark = Mark::Points(CentiPoints::new(31200));
    assert_eq!(Measure::of(&mark), Some(Measure::Points));
}

#[test]
fn measure_of_raw_is_none() {
    let mark = Mark::Raw("unparsed".to_string());
    assert_eq!(Measure::of(&mark), None);
}

#[test]
fn measure_time_value() -> TestResult {
    let mark = Mark::TimeSeconds(ExactSeconds::parse("10.94")?);
    check!(
        eq;
        Measure::Time.value(&mark),
        Some(10_940_000_000)
    );
    Ok(())
}

#[test]
fn measure_distance_value_metric_cm() {
    let mark = Mark::DistanceMetres(CentiMetres::new(642));
    assert_eq!(Measure::Distance.value(&mark), Some(6_420_000i64));
}

#[test]
fn measure_points_value() {
    let mark = Mark::Points(CentiPoints::new(31200));
    assert_eq!(Measure::Points.value(&mark), Some(31200i64));
}

#[test]
fn measure_field_imperial_exact_micrometres() {
    let mark = Mark::FieldImperial {
        feet_mark: "20-00.00".to_string(),
        metres: CentiMetres::new(609),
    };
    assert_eq!(Measure::Distance.value(&mark), Some(6_096_000i64));
}

#[test]
fn measure_field_imperial_hundredth_inch_exact() {
    let mark = Mark::FieldImperial {
        feet_mark: "3-0.01".to_string(),
        metres: CentiMetres::new(91),
    };
    assert_eq!(Measure::Distance.value(&mark), Some(914_654i64));
}

#[test]
fn measure_value_type_mismatch() {
    let mark = Mark::DistanceMetres(CentiMetres::new(642));
    assert_eq!(Measure::Time.value(&mark), None);
}

#[test]
fn measure_time_better_is_lower() {
    assert!(Measure::Time.better(10_800_000_000, 10_940_000_000));
    assert!(!Measure::Time.better(10_940_000_000, 10_800_000_000));
}

#[test]
fn measure_distance_better_is_higher() {
    assert!(Measure::Distance.better(7_000_000, 6_420_000));
    assert!(!Measure::Distance.better(6_420_000, 7_000_000));
}

#[test]
fn measure_points_better_is_higher() {
    assert!(Measure::Points.better(32000, 31200));
    assert!(!Measure::Points.better(31200, 32000));
}

#[test]
fn measure_normalized_time() -> TestResult {
    let mark = Mark::TimeSeconds(ExactSeconds::parse("10.94")?);
    check!(
        eq;
        Measure::Time.normalized_mark(&mark),
        Some(10.94)
    );
    Ok(())
}

#[test]
fn measure_normalized_distance() {
    let mark = Mark::DistanceMetres(CentiMetres::new(642));
    assert_eq!(Measure::Distance.normalized_mark(&mark), Some(6.42));
}

#[test]
fn measure_normalized_points() {
    let mark = Mark::Points(CentiPoints::new(31200));
    assert_eq!(Measure::Points.normalized_mark(&mark), Some(312.0));
}

#[test]
fn measure_normalized_raw_is_none() {
    let mark = Mark::Raw("unparsed".to_string());
    assert_eq!(Measure::Time.normalized_mark(&mark), None);
}

#[test]
fn canary_two_four_feet_beats_three_feet_and_three_quarters(
) -> Result<(), Box<dyn std::error::Error>> {
    let lower = field_micrometres("3-0.75").ok_or("three feet and three quarters")?;
    let higher = field_micrometres("4-0.00").ok_or("four feet")?;
    check!(higher > lower);
    check!(Measure::Distance.better(higher, lower));
    check!(!Measure::Distance.better(lower, higher));
    Ok(())
}

#[test]
fn canary_three_quarter_inch_distinction_is_retained() -> Result<(), Box<dyn std::error::Error>> {
    let four = field_micrometres("5-4.00").ok_or("five feet four")?;
    let quarter = field_micrometres("5-4.25").ok_or("five feet four and a quarter")?;
    check!(eq; quarter.checked_sub(four), Some(6_350));
    Ok(())
}

#[test]
fn canary_four_half_inch_and_fifty_hundredths_are_exactly_equal(
) -> Result<(), Box<dyn std::error::Error>> {
    check!(
        eq;
        field_micrometres("0-0.5"),
        field_micrometres("0-0.50")
    );
    Ok(())
}
