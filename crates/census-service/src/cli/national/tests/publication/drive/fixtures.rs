use super::*;

pub(super) const INVOCATION: &str = "inv_1aiqX0vFEFNH1Umgre58JiCLgHfTtztYK5";

pub(super) fn accepted() -> (&'static str, String) {
    (
        "202 Accepted",
        serde_json::json!({"invocationId": INVOCATION, "status": "Accepted"}).to_string(),
    )
}

pub(super) fn selected_prefix() -> TestResult<Vec<(&'static str, String)>> {
    Ok(vec![
        ("200 OK", bound_request(&["WI"])?),
        ("200 OK", bound_history(Some("2026-10-01"))?),
        ("200 OK", capabilities()?),
        accepted(),
    ])
}

pub(super) fn next_cycle(report: &NationalReport) -> TestResult<Vec<(&'static str, String)>> {
    let mut responses = vec![("200 OK", "{\"status\":\"unbound_legacy\"}".into())];
    responses.extend(selected_prefix()?);
    responses.push(ready(report)?);
    Ok(responses)
}

pub(super) fn complete_report() -> TestResult<NationalReport> {
    Ok(serde_json::from_value(serde_json::json!({
        "season": 2026, "revision": 1, "completeness": "final",
        "jurisdictions": [{
            "jurisdiction": "WI", "identity": "retained-jurisdiction",
            "rosters_total": 0, "rosters_committed": 0, "rosters_remaining": 0,
            "rosters_skipped": 0, "blocked": false, "athletes": 0, "class_of_2027": 0
        }],
        "failures": [], "owed": [], "rosters_total": 0, "athletes_total": 0,
        "class_of_2027_total": 0, "today": "2026-10-14"
    }))?)
}

pub(super) fn failed_report() -> TestResult<NationalReport> {
    let mut report = complete_report()?;
    report.jurisdictions.clear();
    report
        .failures
        .push(census_service::restate_services::NationalFailure {
            jurisdiction: census_domain::UsJurisdiction::Wisconsin,
            identity: "retained-jurisdiction".into(),
            error: "retained failure".into(),
        });
    Ok(report)
}

pub(super) fn owed_report() -> TestResult<NationalReport> {
    let mut report = complete_report()?;
    report
        .owed
        .push(census_service::restate_services::JurisdictionOwed {
            jurisdiction: census_domain::UsJurisdiction::Wisconsin,
            identity: "retained-jurisdiction".into(),
            stages_run: vec!["rosters".into()],
            owed: vec!["rosters".into()],
            reasons: vec!["retained unfinished rosters".into()],
        });
    Ok(report)
}

pub(super) fn ready(report: &NationalReport) -> TestResult<(&'static str, String)> {
    Ok(("200 OK", serde_json::to_string(report)?))
}

pub(super) fn workflow(ordinal: Option<u32>) -> TestResult<String> {
    let season = census_domain::model::SchoolYear::new(2026).ok_or("fixture season")?;
    let jurisdictions = &[census_domain::UsJurisdiction::Wisconsin];
    let identity = match ordinal {
        None => WorkflowIdentity::national(season, Revision(1), jurisdictions),
        Some(ordinal) => WorkflowIdentity::national_resume(
            season,
            Revision(1),
            jurisdictions,
            std::num::NonZeroU32::new(ordinal).ok_or("fixture ordinal")?,
        ),
    };
    Ok(identity.as_str().to_owned())
}

pub(super) fn failure(outcome: anyhow::Result<()>) -> TestResult<String> {
    let error = outcome.err().ok_or("drive unexpectedly succeeded")?;
    Ok(format!("{error:#}"))
}

pub(super) fn send_paths(captured: &[(String, Vec<u8>)]) -> Vec<&str> {
    captured
        .iter()
        .filter_map(|(path, _)| path.contains("/run/send ").then_some(path.as_str()))
        .collect()
}

pub(super) fn output_path(path: &str) -> bool {
    path == format!("GET /restate/invocation/{INVOCATION}/output HTTP/1.1\r\n")
}
