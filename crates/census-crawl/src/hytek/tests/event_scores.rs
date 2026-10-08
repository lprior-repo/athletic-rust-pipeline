use super::*;
use census_domain::model::{CentiMetres, CentiPoints};

fn report(label: &str, mark: &str) -> Result<ParsedMeet, String> {
    let lines = vec![
        "Adapter Regression Meet - 5/27/2026".to_string(),
        format!("Boys {label}"),
        format!(
            "    {:<20} {:>4} {:<20} {:>24}",
            "Name", "Year", "School", "Finals"
        ),
        format!(
            "  1 {:<20} {:>4} {:<20} {:>24}",
            "Test Athlete", 11, "Test School", mark
        ),
    ];
    parse(&lines, source()).ok_or_else(|| "synthetic report was not parsed".to_string())
}

#[test]
fn qualified_hurdle_reports_preserve_event_identity_and_times() -> TestResult {
    for (label, expected, token, centiseconds) in [
        (
            "Varsity 110 Meter Hurdles Finals",
            EventKind::Track110mHurdles,
            "14.25",
            1425,
        ),
        (
            "Varsity 400 Meter Hurdles Finals",
            EventKind::Track400mHurdles,
            "54.25",
            5425,
        ),
        (
            "Varsity 400m Hurdles Finals",
            EventKind::Track400mHurdles,
            "54.25",
            5425,
        ),
        ("110H", EventKind::Track110mHurdles, "14.25", 1425),
        ("400H", EventKind::Track400mHurdles, "54.25", 5425),
    ] {
        let meet = report(label, token)?;
        check!(eq; (meet.rows_parsed, meet.rows_skipped), (1, 0), "{label}");
        check!(eq; meet.events.len(), 1);
        let event = &meet.events[0];
        check!(eq; event.label, label);
        check!(eq; event.kind, expected, "{label}");
        check!(eq;
            event.rows[0].mark,
            Mark::TimeSeconds(ExactSeconds::from_parts(i64::from(centiseconds).checked_mul(10_000_000).ok_or("fixture time overflow")?, 2)?)
        );
    }
    Ok(())
}

#[test]
fn combined_reports_parse_numeric_totals_as_points_including_range_edges() -> TestResult {
    for (label, expected) in [
        ("Varsity Decathlon Finals", EventKind::Decathlon),
        ("Varsity Pentathlon Finals", EventKind::Pentathlon),
        ("Varsity Heptathlon Finals", EventKind::Heptathlon),
    ] {
        for (token, centipoints) in [
            ("0", 0),
            ("3456", 345600),
            ("3120.25", 312025),
            ("3456.125", 345613),
            ("21474836.47", i32::MAX),
        ] {
            let meet = report(label, token)?;
            check!(eq;
                (meet.rows_parsed, meet.rows_skipped),
                (1, 0),
                "{label}: {token}"
            );
            let event = &meet.events[0];
            check!(eq; event.kind, expected);
            check!(eq;
                event.rows[0].mark,
                Mark::Points(CentiPoints::new(centipoints)),
                "{label}: {token}"
            );
            check!(eq; event.rows[0].timing, None);
            check!(eq; event.rows[0].wind_mps, None);
            check!(eq; event.rows[0].points, None);
        }
    }
    Ok(())
}

#[test]
fn combined_reports_skip_malformed_or_out_of_range_scores() -> TestResult {
    for label in ["Decathlon", "Pentathlon", "Heptathlon"] {
        for token in [
            "",
            "-1",
            "-0.01",
            "NaN",
            "inf",
            "-inf",
            "1e309",
            "1:23.45",
            "13-9",
            "3456pts",
            "21474836.48",
            "21474836.475",
            "99999999999999999999999",
        ] {
            let meet = report(label, token)?;
            check!(eq;
                (meet.rows_parsed, meet.rows_skipped),
                (0, 1),
                "{label}: {token}"
            );
            check!(eq; meet.events[0].rows, Vec::new());
        }
    }
    Ok(())
}

#[test]
fn ordinary_field_reports_keep_distance_marks() -> TestResult {
    for (label, token, expected) in [
        (
            "Shot Put",
            "12.34",
            Mark::DistanceMetres(CentiMetres::new(1234)),
        ),
        (
            "Long Jump",
            "13-9",
            Mark::FieldImperial {
                feet_mark: "13-9".to_string(),
                metres: CentiMetres::new(419),
            },
        ),
    ] {
        let meet = report(label, token)?;
        check!(eq; (meet.rows_parsed, meet.rows_skipped), (1, 0));
        check!(eq; meet.events[0].rows[0].mark, expected);
    }
    Ok(())
}

#[test]
fn combined_reports_preserve_no_mark_outcomes() -> TestResult {
    for label in ["Decathlon", "Pentathlon", "Heptathlon"] {
        for token in NO_MARK {
            let meet = report(label, token)?;
            check!(eq; (meet.rows_parsed, meet.rows_skipped), (1, 0));
            check!(eq; meet.events[0].rows[0].mark, Mark::Raw(token.to_string()));
        }
    }
    Ok(())
}
