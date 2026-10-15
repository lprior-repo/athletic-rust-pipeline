use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn single_civil_day_requires_valid_complete_published_dates() -> TestResult {
    for (start, end) in [
        ("2026-03-27", Some("2026-03-27")),
        ("2026-03-27T23:30:00-05:00", None),
    ] {
        assert_eq!(
            date(start, end),
            RetainedPerformanceDate::Exact(ProfileDate::parse("2026-03-27")?)
        );
    }
    for (start, end) in [
        ("2026-03-27", Some("2026-03-28")),
        ("2026-03-27", Some("2026-03-26")),
        ("2026-03-27", Some("2026-03-27invalid")),
        ("2026-03-27", Some("2026")),
        ("2026-02-30", None),
        ("2026-03-27invalid", None),
        ("2026", None),
    ] {
        assert_eq!(date(start, end), RetainedPerformanceDate::Unresolved);
    }
    Ok(())
}

#[test]
fn only_exact_owned_division_labels_supply_status() {
    for value in [
        serde_json::json!({"gradYear": 2027, "grade": 11}),
        serde_json::json!({"divisionName": null}),
        serde_json::json!({"divisionName": "High School Open"}),
        serde_json::json!({"teamName": "Example High School"}),
    ] {
        assert_eq!(division(&value), RetainedResultDivision::Unknown);
    }
    assert_eq!(
        division(&serde_json::json!({"divisionName": "High School"})),
        RetainedResultDivision::HighSchool
    );
}
