use super::*;
use crate::school_address::LaneSelection;

#[test]
fn acquisition_url_and_date_apply_to_the_matching_lane_only() -> TestResult {
    let lanes = evidence(
        r#"{"acquisitions":[{"entry":7,"provider":"state-ed","path":"a.html","url":"https://example.org/a","retrieved_at":"2026-09-01"}]}"#,
    )?;
    check!(eq; lane(&lanes, "a.html")?.url.as_deref(), Some("https://example.org/a"));
    check!(eq; lane(&lanes, "a.html")?.observed_on.as_deref(), Some("2026-09-01"));
    check!(eq; lane(&lanes, "b.html")?.url, None);
    check!(eq; lane(&lanes, "b.html")?.observed_on, None);
    Ok(())
}

#[test]
fn acquisition_records_keep_independent_capture_specific_dates() -> TestResult {
    let lanes = evidence(
        r#"{"acquisitions":[{"entry":7,"provider":"state-ed","path":"a.html","url":"https://example.org/default","retrieved_at":"2026-09-02"},{"entry":8,"provider":"state-ed","path":"b.html","url":"https://example.org/default","retrieved_at":"2026-09-03"}]}"#,
    )?;
    check!(eq; lane(&lanes, "a.html")?.url.as_deref(), Some("https://example.org/default"));
    check!(eq; lane(&lanes, "b.html")?.url.as_deref(), Some("https://example.org/default"));
    check!(eq; lane(&lanes, "a.html")?.observed_on.as_deref(), Some("2026-09-02"));
    check!(eq; lane(&lanes, "b.html")?.observed_on.as_deref(), Some("2026-09-03"));
    Ok(())
}

#[test]
fn acquisition_url_and_date_select_independently_per_lane() -> TestResult {
    let lanes = evidence(
        r#"{"acquisitions":[{"entry":7,"provider":"state-ed","path":"a.html","url":"https://example.org/a","retrieved_at":"2026-09-01"},{"entry":8,"provider":"state-ed","path":"b.html","url":"https://example.org/default","retrieved_at":"2026-09-03"}]}"#,
    )?;
    check!(eq; lanes.lanes("state-ed").len(), 2);
    let alpha = lanes
        .select("state-ed", &key("1")?)
        .map_err(|error| format!("{error:?}"))?;
    check!(eq; alpha.url.as_deref(), Some("https://example.org/a"));
    check!(eq; alpha.observed_on.as_deref(), Some("2026-09-01"));
    let beta = lanes
        .select("state-ed", &key("2")?)
        .map_err(|error| format!("{error:?}"))?;
    check!(eq; beta.url.as_deref(), Some("https://example.org/default"));
    check!(eq; beta.observed_on.as_deref(), Some("2026-09-03"));
    Ok(())
}

#[test]
fn unmatched_acquisition_paths_are_refused_without_matching_a_lane() -> TestResult {
    for input in [
        r#"{"acquisitions":[{"entry":7,"provider":"state-ed","path":"unknown.html","url":"https://example.org/a"}]}"#,
        r#"{"acquisitions":[{"entry":7,"provider":"state-ed","path":"unknown.html","url":"https://example.org/a","retrieved_at":"2026-09-01"}]}"#,
    ] {
        let overrides: Overrides = serde_json::from_str(input)?;
        check!(build_lane_evidence(&report()?, &overrides).is_err());
    }
    Ok(())
}

#[test]
fn identical_new_records_are_retained_and_share_one_projection() -> TestResult {
    let overrides = Overrides::from_acquisitions(vec![
        acquisition(91, "a.html", Some("2026-09-01")),
        acquisition(7, "a.html", Some("2026-09-01")),
    ])?;
    let mut report = report()?;
    let extra_key = key("2")?;
    let alpha = report.lanes.first_mut().ok_or("missing alpha lane")?;
    alpha.captured.push(extra_key);
    let mut reversed = alpha.clone();
    reversed.captured.reverse();
    report.lanes.push(reversed);
    let lanes = build_lane_evidence(&report, &overrides)?;
    check!(eq; overrides.acquisitions().ok_or("missing acquisitions")?.len(), 2);
    check!(eq; lanes.lanes("state-ed").iter().filter(|lane| lane.path == "a.html").count(), 1);
    check!(lanes.select("state-ed", &key("1")?).is_ok());
    let encoded = serde_json::to_string(&overrides)?;
    check!(encoded.contains("\"entry\":91"));
    check!(encoded.contains("\"entry\":7"));
    Ok(())
}

#[test]
fn different_new_acquisition_dates_remain_distinct_and_ambiguous() -> TestResult {
    let overrides = Overrides::from_acquisitions(vec![
        acquisition(91, "a.html", Some("2026-09-01")),
        acquisition(7, "a.html", Some("2026-09-02")),
    ])?;
    let lanes = build_lane_evidence(&report()?, &overrides)?;
    check!(eq; lanes.lanes("state-ed").iter().filter(|lane| lane.path == "a.html").count(), 2);
    check!(matches!(
        lanes.select("state-ed", &key("1")?),
        Err(LaneSelection::Ambiguous { .. })
    ));
    check!(eq; overrides.acquisitions().ok_or("missing acquisitions")?.len(), 2);
    Ok(())
}
