use super::*;
use crate::school_address::LaneSelection;

#[test]
fn legacy_capture_url_uses_provider_wide_date_fallback() -> TestResult {
    let lanes = evidence(
        r#"{"urls":{"state-ed@a.html":"https://example.org/a"},"dates":{"state-ed":"2026-09-01"}}"#,
    )?;
    check!(eq; lane(&lanes, "a.html")?.url.as_deref(), Some("https://example.org/a"));
    check!(eq; lane(&lanes, "a.html")?.observed_on.as_deref(), Some("2026-09-01"));
    check!(eq; lane(&lanes, "b.html")?.url, None);
    check!(eq; lane(&lanes, "b.html")?.observed_on.as_deref(), Some("2026-09-01"));
    Ok(())
}

#[test]
fn legacy_provider_url_uses_independent_capture_specific_dates() -> TestResult {
    let lanes = evidence(
        r#"{"urls":{"state-ed":"https://example.org/default"},"dates":{"state-ed@a.html":"2026-09-02","state-ed@b.html":"2026-09-03"}}"#,
    )?;
    check!(eq; lane(&lanes, "a.html")?.url.as_deref(), Some("https://example.org/default"));
    check!(eq; lane(&lanes, "b.html")?.url.as_deref(), Some("https://example.org/default"));
    check!(eq; lane(&lanes, "a.html")?.observed_on.as_deref(), Some("2026-09-02"));
    check!(eq; lane(&lanes, "b.html")?.observed_on.as_deref(), Some("2026-09-03"));
    Ok(())
}

#[test]
fn legacy_exact_url_and_date_take_independent_precedence_once() -> TestResult {
    let lanes = evidence(
        r#"{"urls":{"state-ed":"https://example.org/default","state-ed@a.html":"https://example.org/a"},"dates":{"state-ed":"2026-09-01","state-ed@b.html":"2026-09-03"}}"#,
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
fn legacy_date_only_lane_evidence_is_retained_without_invented_url() -> TestResult {
    let input = r#"{"dates":{"state-ed@a.html":"2026-09-02"}}"#;
    let overrides: Overrides = serde_json::from_str(input)?;
    let lanes = build_lane_evidence(&report()?, &overrides)?;
    check!(eq; lane(&lanes, "a.html")?.url, None);
    check!(eq; lane(&lanes, "a.html")?.observed_on.as_deref(), Some("2026-09-02"));
    check!(eq; serde_json::to_string(&overrides)?, r#"{"urls":{},"dates":{"state-ed@a.html":"2026-09-02"}}"#);
    Ok(())
}

#[test]
fn qualified_url_and_date_selectors_must_match_literal_generation_paths() -> TestResult {
    for input in [
        r#"{"urls":{"state-ed@unknown.html":"https://example.org/a"}}"#,
        r#"{"dates":{"state-ed@unknown.html":"2026-09-01"}}"#,
        r#"{"urls":{"state-ed@./a.html":"https://example.org/a"}}"#,
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

#[test]
fn legacy_and_new_inputs_produce_the_same_owned_lane_tuple() -> TestResult {
    let legacy = evidence(
        r#"{"urls":{"state-ed@a.html":"https://example.org/a","state-ed@b.html":"https://example.org/a"},"dates":{"state-ed":"2026-09-01"}}"#,
    )?;
    let current = Overrides::from_acquisitions(vec![
        acquisition(91, "a.html", Some("2026-09-01")),
        acquisition(7, "b.html", Some("2026-09-01")),
    ])?;
    let new = build_lane_evidence(&report()?, &current)?;
    check!(eq; serde_json::to_vec(&legacy)?, serde_json::to_vec(&new)?);
    Ok(())
}
