use super::*;
use census_reconcile::identity::{Revision, WorkflowIdentity};
use census_service::restate_services::{NationalCompleteness, NationalReport};

#[path = "drive/fixtures.rs"]
mod fixtures;
use fixtures::*;

#[tokio::test]
async fn national_drive_observation_errors_retain_selected_workflow_and_invocation() -> TestResult {
    for response in [
        absent(),
        (
            "500 Internal Server Error",
            "{\"message\":\"publication still running\"}".into(),
        ),
        ("200 OK", "malformed report".into()),
        ("200 OK\r\ninvalid header line", String::new()),
    ] {
        let mut responses = selected_prefix()?;
        responses.push(response);
        let (captured, outcome) =
            exchange_mode(&["--drive", "--resume", "7"], responses, false).await?;
        check!(eq; captured.len(), 5);
        check!(eq; send_paths(&captured).len(), 1);
        check!(send_paths(&captured)
            .first()
            .is_some_and(|path| path.contains("%3Aresume%3A7/run/send")));
        check!(captured.last().is_some_and(|(path, _)| output_path(path)));
        let diagnostic = failure(outcome)?;
        check!(diagnostic.contains(&workflow(Some(7))?), "{diagnostic}");
        check!(diagnostic.contains(INVOCATION), "{diagnostic}");
        check!(
            diagnostic.contains("same command to reattach"),
            "{diagnostic}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn national_drive_initial_attachment_errors_retain_resolved_invocation_and_workflow(
) -> TestResult {
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
        let (captured, outcome) = exchange_mode(&["--drive"], responses, false).await?;
        check!(eq; captured.len(), 5);
        check!(eq; send_paths(&captured).len(), 1);
        check!(captured.last().is_some_and(|(path, _)| output_path(path)));
        let diagnostic = failure(outcome)?;
        check!(diagnostic.contains(&workflow(None)?), "{diagnostic}");
        check!(diagnostic.contains(INVOCATION), "{diagnostic}");
    }
    Ok(())
}

#[tokio::test]
async fn national_drive_observation_bound_retains_original_workflow_without_report_probe(
) -> TestResult {
    let (captured, outcome) = exchange_mode(
        &["--drive", "--timeout-seconds", "0"],
        vec![("200 OK", capabilities()?), accepted()],
        false,
    )
    .await?;
    check!(eq; captured.len(), 2);
    check!(eq; send_paths(&captured).len(), 1);
    check!(!captured
        .iter()
        .any(|(path, _)| path.contains("%3Aresume%3A") || path.contains("/report ")));
    let diagnostic = failure(outcome)?;
    check!(diagnostic.contains(&workflow(None)?), "{diagnostic}");
    check!(diagnostic.contains(INVOCATION), "{diagnostic}");
    check!(
        diagnostic.contains("observation bound of 0 seconds reached"),
        "{diagnostic}"
    );
    Ok(())
}

#[tokio::test]
async fn national_drive_not_ready_output_never_uses_a_shared_final_report_to_resume() -> TestResult
{
    let responses = vec![
        ("200 OK", capabilities()?),
        accepted(),
        ("470 Not Ready", String::new()),
        ready(&failed_report()?)?,
    ];
    let (captured, outcome) =
        exchange_mode(&["--drive", "--timeout-seconds", "10"], responses, false).await?;
    check!(eq; captured.len(), 3);
    check!(eq; send_paths(&captured).len(), 1);
    check!(captured.last().is_some_and(|(path, _)| output_path(path)));
    check!(!captured.iter().any(|(path, _)| path.contains("/report ")));
    let diagnostic = failure(outcome)?;
    check!(
        diagnostic.contains("observation bound of 10 seconds reached"),
        "{diagnostic}"
    );
    check!(diagnostic.contains(&workflow(None)?), "{diagnostic}");
    check!(diagnostic.contains(INVOCATION), "{diagnostic}");
    Ok(())
}

#[tokio::test]
async fn national_drive_partial_ready_is_protocol_refusal_even_with_owed_work() -> TestResult {
    for mut report in [complete_report()?, failed_report()?, owed_report()?] {
        report.completeness = NationalCompleteness::Partial;
        for arguments in [vec![], vec!["--drive"]] {
            let responses = vec![("200 OK", capabilities()?), accepted(), ready(&report)?];
            let (captured, outcome) = exchange_mode(&arguments, responses, false).await?;
            check!(eq; captured.len(), 3);
            check!(eq; send_paths(&captured).len(), 1);
            let diagnostic = failure(outcome)?;
            check!(
                diagnostic.contains("Ready returned a Partial report"),
                "{diagnostic}"
            );
            check!(diagnostic.contains(&workflow(None)?), "{diagnostic}");
            check!(diagnostic.contains(INVOCATION), "{diagnostic}");
        }
    }
    Ok(())
}

#[tokio::test]
async fn national_drive_attachment_probe_refuses_partial_ready_before_detaching() -> TestResult {
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
async fn national_drive_final_failure_or_owed_work_starts_resume_two_after_ready() -> TestResult {
    for report in [failed_report()?, owed_report()?] {
        let mut responses = vec![("200 OK", capabilities()?), accepted(), ready(&report)?];
        responses.extend(next_cycle(&complete_report()?)?);
        let (captured, outcome) = exchange_mode(&["--drive"], responses, false).await?;
        outcome?;
        check!(eq; captured.len(), 9);
        let paths = send_paths(&captured);
        check!(eq; paths.len(), 2);
        check!(paths
            .first()
            .is_some_and(|path| !path.contains("%3Aresume%3A")));
        check!(paths
            .get(1)
            .is_some_and(|path| path.contains("%3Aresume%3A2/run/send")));
        check!(captured.get(2).is_some_and(|(path, _)| output_path(path)));
        check!(captured
            .get(3)
            .is_some_and(|(path, _)| path.contains("%3Aresume%3A2/request")));
    }
    Ok(())
}

#[tokio::test]
async fn national_drive_selected_resume_preserves_bound_request_and_budgets() -> TestResult {
    let mut responses = selected_prefix()?;
    responses.push(ready(&owed_report()?)?);
    responses.extend(next_cycle(&complete_report()?)?);
    let (captured, outcome) = exchange_mode(
        &["--drive", "--resume", "7", "--pass-budget", "999"],
        responses,
        false,
    )
    .await?;
    outcome?;
    check!(eq; captured.len(), 11);
    let sends = captured
        .iter()
        .filter(|(path, _)| path.contains("/run/send "))
        .collect::<Vec<_>>();
    check!(eq; sends.len(), 2);
    check!(sends
        .first()
        .is_some_and(|(path, _)| path.contains("%3Aresume%3A7/run/send")));
    check!(sends
        .get(1)
        .is_some_and(|(path, _)| path.contains("%3Aresume%3A8/run/send")));
    let bound: serde_json::Value = serde_json::from_str(&bound_request(&["WI"])?)?;
    let request = bound.get("request").ok_or("fixture bound request absent")?;
    for (_, body) in sends {
        check!(eq; serde_json::from_slice::<serde_json::Value>(body)?, *request);
    }
    Ok(())
}

#[tokio::test]
async fn national_drive_final_output_requires_explicit_drive_and_owed_work() -> TestResult {
    for (arguments, report, success) in [
        (vec![], failed_report()?, false),
        (vec![], owed_report()?, false),
        (vec!["--drive"], complete_report()?, true),
    ] {
        let responses = vec![("200 OK", capabilities()?), accepted(), ready(&report)?];
        let (captured, outcome) = exchange_mode(&arguments, responses, false).await?;
        check!(eq; captured.len(), 3);
        check!(eq; send_paths(&captured).len(), 1);
        check!(eq; outcome.is_ok(), success);
    }
    Ok(())
}

#[tokio::test]
async fn national_drive_cycle_exhaustion_does_not_allocate_another_ordinal() -> TestResult {
    let report = owed_report()?;
    let mut responses = vec![("200 OK", capabilities()?), accepted(), ready(&report)?];
    for _ in 1..crate::cli::national::DRIVE_CYCLES {
        responses.extend(next_cycle(&report)?);
    }
    let (captured, outcome) = exchange_mode(&["--drive"], responses, false).await?;
    let paths = send_paths(&captured);
    check!(eq; paths.len(), usize::try_from(crate::cli::national::DRIVE_CYCLES)?);
    check!(paths
        .last()
        .is_some_and(|path| path.contains("%3Aresume%3A96/run/send")));
    check!(captured.last().is_some_and(|(path, _)| output_path(path)));
    check!(failure(outcome)?.contains("drive cycles exhausted with the run still owed work"));
    Ok(())
}

#[tokio::test]
async fn national_drive_completed_output_refuses_ordinal_overflow_without_new_key() -> TestResult {
    let mut responses = selected_prefix()?;
    responses.push(ready(&failed_report()?)?);
    let (captured, outcome) =
        exchange_mode(&["--drive", "--resume", "4294967295"], responses, false).await?;
    check!(eq; captured.len(), 5);
    check!(eq; send_paths(&captured).len(), 1);
    check!(failure(outcome)?.contains("resume ordinal exhausted"));
    Ok(())
}
