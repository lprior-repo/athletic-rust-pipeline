use super::*;
use census_domain::model::Revision;
use census_domain::national_census::{NationalCompleteness, NationalReport};

#[path = "drive/fixtures.rs"]
mod fixtures;
use fixtures::*;

#[path = "drive/notice.rs"]
mod notice;

#[tokio::test]
async fn national_observation_errors_retain_selected_workflow_and_invocation() -> TestResult {
    for response in [
        absent(),
        (
            "500 Internal Server Error",
            "{\"message\":\"publication still running\"}".into(),
        ),
        ("200 OK", "malformed report".into()),
        ("200 OK\r\ninvalid header line", String::new()),
    ] {
        let mut responses = fresh_prefix()?;
        responses.push(response);
        let (captured, outcome) = exchange_mode(&[], responses).await?;
        check!(eq; captured.len(), 3);
        check!(eq; send_paths(&captured).len(), 1);
        check!(send_paths(&captured)
            .first()
            .is_some_and(|path| { path.contains("/run/send") && !path.contains("%3Aresume%3A") }));
        check!(captured.last().is_some_and(|(path, _)| output_path(path)));
        let diagnostic = failure(outcome)?;
        check!(diagnostic.contains(&workflow()?), "{diagnostic}");
        check!(diagnostic.contains(INVOCATION), "{diagnostic}");
        check!(
            diagnostic.contains("same command to reattach"),
            "{diagnostic}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn national_initial_attachment_errors_retain_resolved_invocation_and_workflow() -> TestResult
{
    for response in [
        absent(),
        (
            "500 Internal Server Error",
            "{\"message\":\"retained publication failure\"}".into(),
        ),
        ("200 OK", "malformed report".into()),
    ] {
        let responses = vec![
            ("200 OK", capabilities()?),
            absent(),
            ("200 OK", bound_request(&["WI"])?),
            (
                "200 OK",
                serde_json::json!({"invocationId": INVOCATION}).to_string(),
            ),
            response,
        ];
        let (captured, outcome) = exchange_mode(&[], responses).await?;
        check!(eq; captured.len(), 5);
        check!(eq; send_paths(&captured).len(), 1);
        check!(captured.last().is_some_and(|(path, _)| output_path(path)));
        let diagnostic = failure(outcome)?;
        check!(diagnostic.contains(&workflow()?), "{diagnostic}");
        check!(diagnostic.contains(INVOCATION), "{diagnostic}");
    }
    Ok(())
}

#[tokio::test]
async fn national_observation_bound_retains_original_workflow_without_report_probe() -> TestResult {
    let (captured, outcome) = exchange_mode(
        &["--timeout-seconds", "0"],
        vec![("200 OK", capabilities()?), accepted()],
    )
    .await?;
    check!(eq; captured.len(), 2);
    check!(eq; send_paths(&captured).len(), 1);
    check!(!captured
        .iter()
        .any(|(path, _)| path.contains("%3Aresume%3A") || path.contains("/report ")));
    let diagnostic = failure(outcome)?;
    check!(diagnostic.contains(&workflow()?), "{diagnostic}");
    check!(diagnostic.contains(INVOCATION), "{diagnostic}");
    check!(
        diagnostic.contains("observation bound of 0 seconds reached"),
        "{diagnostic}"
    );
    Ok(())
}

#[tokio::test]
async fn national_not_ready_output_never_uses_a_shared_final_report_to_resume() -> TestResult {
    let responses = vec![
        ("200 OK", capabilities()?),
        accepted(),
        ("470 Not Ready", String::new()),
        ready(&failed_report()?)?,
    ];
    let (captured, outcome) = exchange_mode(&["--timeout-seconds", "10"], responses).await?;
    check!(eq; captured.len(), 3);
    check!(eq; send_paths(&captured).len(), 1);
    check!(captured.last().is_some_and(|(path, _)| output_path(path)));
    check!(!captured.iter().any(|(path, _)| path.contains("/report ")));
    let diagnostic = failure(outcome)?;
    check!(
        diagnostic.contains("observation bound of 10 seconds reached"),
        "{diagnostic}"
    );
    check!(diagnostic.contains(&workflow()?), "{diagnostic}");
    check!(diagnostic.contains(INVOCATION), "{diagnostic}");
    Ok(())
}

#[tokio::test]
async fn national_partial_ready_is_protocol_refusal_even_with_owed_work() -> TestResult {
    for mut report in [complete_report()?, failed_report()?, owed_report()?] {
        report.completeness = NationalCompleteness::Partial;
        let mut responses = fresh_prefix()?;
        responses.push(ready(&report)?);
        let (captured, outcome) = exchange_mode(&[], responses).await?;
        check!(eq; captured.len(), 3);
        check!(eq; send_paths(&captured).len(), 1);
        let diagnostic = failure(outcome)?;
        check!(
            diagnostic.contains("Ready returned a Partial report"),
            "{diagnostic}"
        );
        check!(diagnostic.contains(&workflow()?), "{diagnostic}");
        check!(diagnostic.contains(INVOCATION), "{diagnostic}");
    }
    Ok(())
}

#[tokio::test]
async fn national_attachment_probe_refuses_partial_ready_before_detaching() -> TestResult {
    let mut report = complete_report()?;
    report.completeness = NationalCompleteness::Partial;
    let responses = vec![
        ("200 OK", capabilities()?),
        absent(),
        ("200 OK", bound_request(&["WI"])?),
        (
            "200 OK",
            serde_json::json!({"invocationId": INVOCATION}).to_string(),
        ),
        ready(&report)?,
    ];
    let (captured, outcome) = exchange(&[], responses).await?;
    check!(eq; captured.len(), 5);
    check!(eq; send_paths(&captured).len(), 1);
    check!(failure(outcome)?.contains("Ready returned a Partial report"));
    Ok(())
}

#[tokio::test]
async fn national_final_failure_or_owed_work_reports_without_client_redrive() -> TestResult {
    for report in [failed_report()?, owed_report()?] {
        let responses = vec![("200 OK", capabilities()?), accepted(), ready(&report)?];
        let (captured, outcome) = exchange_mode(&[], responses).await?;
        let diagnostic = failure(outcome)?;
        check!(eq; captured.len(), 3);
        check!(eq; send_paths(&captured).len(), 1);
        check!(!captured
            .iter()
            .any(|(path, _)| path.contains("%3Aresume%3A")));
        check!(
            diagnostic.contains("jurisdiction(s) failed"),
            "{diagnostic}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn national_final_output_reports_completion_failure_or_owed_without_redrive() -> TestResult {
    for (report, success) in [
        (failed_report()?, false),
        (owed_report()?, false),
        (complete_report()?, true),
    ] {
        let mut responses = vec![("200 OK", capabilities()?), accepted(), ready(&report)?];
        if success {
            responses.push(("200 OK", "null".into()));
        }
        let (captured, outcome) = exchange_mode(&[], responses).await?;
        check!(eq; captured.len(), if success { 4 } else { 3 });
        check!(eq; send_paths(&captured).len(), 1);
        check!(!captured
            .iter()
            .any(|(path, _)| path.contains("%3Aresume%3A")));
        check!(eq; outcome.is_ok(), success);
    }
    Ok(())
}
