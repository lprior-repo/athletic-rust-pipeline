use super::*;
use census_domain::model::CentiPoints;

fn synthetic_document(label: &str, mark: &str) -> TestResult<Value> {
    let mut document = document()?;
    document["data"][0]["eventName"] = json!(label);
    document["data"][0]["mark"] = json!(mark);
    Ok(document)
}

#[test]
fn qualified_owned_hurdles_preserve_kind_time_and_provider_json() -> TestResult {
    for (label, expected, token, centiseconds) in [
        (
            "Boys Varsity 110 Meter Hurdles Finals",
            EventKind::Track110mHurdles,
            "14.25",
            1425,
        ),
        (
            "Girls Varsity 400 Meter Hurdles Finals",
            EventKind::Track400mHurdles,
            "54.25",
            5425,
        ),
        (
            "Girls Varsity 400m Hurdles Finals",
            EventKind::Track400mHurdles,
            "54.25",
            5425,
        ),
        ("110H", EventKind::Track110mHurdles, "14.25", 1425),
        ("400H", EventKind::Track400mHurdles, "54.25", 5425),
    ] {
        let document = synthetic_document(label, token)?;
        let page = page(&serde_json::to_vec(&document)?)?;
        check!(eq; page.rejected, Vec::new(), "{label}");
        let row = &page.rows[0];
        check!(eq; row.event_kind, expected, "{label}");
        check!(eq; row.mark, Mark::TimeSeconds(CentiSeconds::new(centiseconds)));
        check!(eq; row.provider, document["data"][0]);
    }
    Ok(())
}

#[test]
fn combined_owned_totals_are_points_with_no_invented_timing_or_provider_changes() -> TestResult {
    for (label, expected) in [
        ("Girls Varsity Decathlon Finals", EventKind::Decathlon),
        ("Girls Varsity Pentathlon Finals", EventKind::Pentathlon),
        ("Girls Varsity Heptathlon Finals", EventKind::Heptathlon),
    ] {
        for (token, centipoints) in [
            ("0", 0),
            ("3456", 345600),
            ("3120.25", 312025),
            ("3456.125", 345613),
            ("21474836.47", i32::MAX),
        ] {
            let document = synthetic_document(label, token)?;
            let page = page(&serde_json::to_vec(&document)?)?;
            check!(eq; page.rejected, Vec::new(), "{label}: {token}");
            let row = &page.rows[0];
            check!(eq; row.event_kind, expected);
            check!(eq;
                row.mark,
                Mark::Points(CentiPoints::new(centipoints)),
                "{label}: {token}"
            );
            check!(eq; row.timing, None);
            check!(eq; row.provider, document["data"][0]);
            check!(eq; page.completeness, OwnedCompleteness::Unknown);
        }
    }
    Ok(())
}

#[test]
fn malformed_owned_scores_are_located_rejections_without_losing_other_results() -> TestResult {
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
            let document = synthetic_document(label, token)?;
            let page = page(&serde_json::to_vec(&document)?)?;
            check!(eq; page.published_rows, 3);
            check!(eq;
                page.rows
                    .iter()
                    .map(|row| row.result_id)
                    .collect::<Vec<_>>(),
                vec![201782277, 201782806],
                "{label}: {token}"
            );
            check!(eq;
                page.rejected,
                vec![OwnedRejection {
                    locator: "data[0]".to_string(),
                    kind: OwnedRejectionKind::InvalidContext,
                    detail: "malformed published mark".to_string(),
                }],
                "{label}: {token}"
            );
            check!(eq; page.rows[0].provider, document["data"][1]);
            check!(eq; page.rows[1].provider, document["data"][2]);
        }
    }
    Ok(())
}

#[test]
fn ordinary_owned_fields_still_parse_metric_and_imperial_distances() -> TestResult {
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
        let document = synthetic_document(label, token)?;
        let page = page(&serde_json::to_vec(&document)?)?;
        check!(eq; page.rejected, Vec::new());
        check!(eq; page.rows[0].mark, expected);
        check!(eq; page.rows[0].provider, document["data"][0]);
    }
    Ok(())
}

#[test]
fn combined_owned_no_mark_outcomes_remain_raw() -> TestResult {
    for label in ["Decathlon", "Pentathlon", "Heptathlon"] {
        for token in crate::hytek::NO_MARK.into_iter().chain(["NT"]) {
            let document = synthetic_document(label, token)?;
            let page = page(&serde_json::to_vec(&document)?)?;
            check!(eq; page.rejected, Vec::new());
            check!(eq; page.rows[0].mark, Mark::Raw(token.to_string()));
            check!(eq; page.rows[0].timing, None);
            check!(eq; page.rows[0].provider, document["data"][0]);
        }
    }
    Ok(())
}
