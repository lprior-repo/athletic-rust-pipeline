use super::{combine, comparison, Cell};
use census_domain::model::performance_comparison::{
    ComparisonPolicy, Measure, PrKey, SurfaceClass, TimingClass, WindClass,
};
use census_domain::model::{AthleteId, EventKind, EventSpecification};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn best_groups_count_utf16_and_line_separators_without_truncation() -> TestResult {
    let first = format!("{}x", "😀".repeat(16_382));
    let expected = format!("{first}\ny");
    check!(eq; expected.encode_utf16().count(), 32_767);
    check!(eq; combine([Ok(first.clone()), Ok("y".to_owned())].into_iter())?, Cell::text(expected));
    check!(combine([Ok(first), Ok("yy".to_owned())].into_iter()).is_err());
    let oversized = "😀".repeat(16_384);
    check!(combine(std::iter::once(Ok(oversized))).is_err());
    Ok(())
}

#[test]
fn identical_marks_in_distinct_comparison_classes_both_survive() -> TestResult {
    let first = PrKey {
        athlete_id: AthleteId::mint("ath", &["synthetic athlete"]),
        event_kind: EventKind::Track400m,
        surface: SurfaceClass::Outdoor,
        wind_class: WindClass::NotApplicable,
        timing: TimingClass::Fat,
        measure: Measure::Time,
        context: None,
        specification: EventSpecification::default(),
        comparison: ComparisonPolicy::Standard,
        adjustment: None,
    };
    let mut second = first.clone();
    second.timing = TimingClass::Hand;
    let rows = [
        comparison("Calculated PR: 50.00".into(), &first),
        comparison("Calculated PR: 50.00".into(), &second),
    ];
    let Cell::Text(text) = combine(rows.into_iter())? else {
        return Err("missing complete groups".into());
    };
    let actual = text
        .lines()
        .map(|line| {
            let (mark, key) = line
                .split_once("; Comparison class: ")
                .ok_or("class boundary")?;
            check!(eq; mark, "Calculated PR: 50.00");
            Ok(serde_json::from_str(key)?)
        })
        .collect::<Result<Vec<serde_json::Value>, Box<dyn std::error::Error>>>()?;
    check!(eq; actual, vec![serde_json::to_value(first)?, serde_json::to_value(second)?]);
    Ok(())
}
