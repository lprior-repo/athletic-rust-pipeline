use super::*;

pub(super) fn assert_lookup(captured: &[(String, Vec<u8>)], index: usize) -> TestResult {
    let season = census_domain::model::SchoolYear::new(2026).ok_or("fixture season")?;
    let revision = census_domain::model::Revision(1);
    let states = &[census_domain::UsJurisdiction::Wisconsin];
    let identity = census_reconcile::identity::WorkflowIdentity::national(season, revision, states);
    let (path, body) = captured.get(index).ok_or("workflow lookup absent")?;
    check!(eq; path, "POST /restate/lookup HTTP/1.1\r\n");
    check!(eq; serde_json::from_slice::<serde_json::Value>(body)?, serde_json::json!({
        "target": "workflow", "workflowName": "NationalCensus", "workflowKey": identity.as_str()
    }));
    check!(eq; captured.iter().filter(|(path, _)| path.contains("/run/send ")).count(), 1);
    Ok(())
}

fn prefix() -> TestResult<Vec<(&'static str, String)>> {
    Ok(vec![
        ("200 OK", capabilities()?),
        absent(),
        ("200 OK", bound_request(&["WI"])?),
    ])
}

#[tokio::test]
async fn managed_attachment_lookup_refusals_never_probe_or_resubmit() -> TestResult {
    for lookup in [
        absent(),
        ("200 OK", "{}".into()),
        (
            "200 OK",
            "{\"invocationId\":\"national:2026-27:wrong:1\"}".into(),
        ),
        ("200 OK", "{\"invocationId\":null}".into()),
        (
            "200 OK",
            "{\"invocationId\":\"inv_abc\",\"workflowKey\":\"foreign\"}".into(),
        ),
        ("200 OK", " ".repeat(4097)),
    ] {
        let mut responses = prefix()?;
        responses.push(lookup);
        let (captured, error) = exercise(&[], responses).await?;
        check!(eq; captured.len(), 4);
        assert_lookup(&captured, 3)?;
        check!(
            error.contains("could not resolve the existing workflow"),
            "{error}"
        );
        check!(!captured.iter().any(|(path, _)| output_path(path)));
    }
    Ok(())
}

#[tokio::test]
async fn managed_attachment_missing_output_refuses_after_lookup_without_new_send() -> TestResult {
    let mut responses = prefix()?;
    responses.extend([resolved(), absent()]);
    let (captured, error) = exercise(&[], responses).await?;
    check!(eq; captured.len(), 5);
    assert_lookup(&captured, 3)?;
    check!(captured
        .get(4)
        .is_some_and(|(path, _)| path
            == &format!("GET /restate/invocation/{RESOLVED_ID}/output HTTP/1.1\r\n")));
    check!(error.contains("no readable invocation exists"), "{error}");
    Ok(())
}

#[tokio::test]
async fn managed_attachment_rejects_output_header_for_another_invocation() -> TestResult {
    let mut responses = prefix()?;
    responses.extend([
        (
            "200 OK",
            "{\"invocationId\":\"inv_AnotherInvocation123\"}".into(),
        ),
        ("200 OK", matching_report().to_string()),
    ]);
    let (captured, error) = exercise(&[], responses).await?;
    check!(eq; captured.len(), 5);
    assert_lookup(&captured, 3)?;
    check!(captured
        .get(4)
        .is_some_and(|(path, _)| path
            == "GET /restate/invocation/inv_AnotherInvocation123/output HTTP/1.1\r\n"));
    check!(
        error.contains("output invocation ID differs from workflow lookup"),
        "{error}"
    );
    Ok(())
}

fn matching_report() -> serde_json::Value {
    serde_json::json!({
        "season": 2026, "revision": 1,
        "jurisdictions": [{
            "jurisdiction": "WI", "identity": "retained-jurisdiction",
            "rosters_total": 0, "rosters_committed": 0, "rosters_remaining": 0,
            "rosters_skipped": 0, "blocked": false, "athletes": 0, "class_of_2027": 0
        }],
        "failures": [], "rosters_total": 0, "athletes_total": 0, "class_of_2027_total": 0,
        "today": "2026-10-14"
    })
}

#[tokio::test]
async fn managed_attachment_accepts_matching_ready_report_and_refuses_foreign_scope() -> TestResult
{
    for (field, value, accepted) in [
        ("season", serde_json::json!(2026), true),
        ("season", serde_json::json!(2025), false),
        ("revision", serde_json::json!(9), false),
        (
            "failures",
            serde_json::json!([{ "jurisdiction": "IA", "identity": "foreign", "error": "foreign" }]),
            false,
        ),
        ("jurisdictions", serde_json::json!([]), false),
    ] {
        let mut ready = matching_report();
        ready
            .as_object_mut()
            .ok_or("report object")?
            .insert(field.into(), value);
        let mut responses = prefix()?;
        responses.extend([resolved(), ("200 OK", ready.to_string())]);
        if accepted {
            responses.extend([("200 OK", ready.to_string()), ("200 OK", "null".into())]);
        }
        let (captured, outcome) = exchange(&[], responses).await?;
        check!(eq; captured.len(), if accepted { 7 } else { 5 });
        assert_lookup(&captured, 3)?;
        check!(eq; outcome.is_ok(), accepted);
        if let Err(error) = outcome {
            check!(
                format!("{error:#}").contains("national attachment report"),
                "{error:#}"
            );
        }
    }
    Ok(())
}

fn successful_report() -> serde_json::Value {
    serde_json::json!({
        "season": 2026, "revision": 1,
        "jurisdictions": [{
            "jurisdiction": "WI", "identity": "retained-jurisdiction",
            "rosters_total": 0, "rosters_committed": 0, "rosters_remaining": 0,
            "rosters_skipped": 0, "blocked": false, "athletes": 0, "class_of_2027": 0
        }],
        "failures": [], "rosters_total": 0, "athletes_total": 0, "class_of_2027_total": 0,
        "today": "2026-10-14"
    })
}

#[tokio::test]
async fn managed_attached_observer_validates_ready_output_without_redriving() -> TestResult {
    let mut foreign_season = successful_report();
    foreign_season["season"] = serde_json::json!(2025);
    let mut foreign_scope = successful_report();
    foreign_scope["jurisdictions"][0]["jurisdiction"] = serde_json::json!("IA");
    let mut foreign_revision = successful_report();
    foreign_revision["revision"] = serde_json::json!(9);
    for (body, accepted) in [
        (successful_report().to_string(), true),
        (foreign_season.to_string(), false),
        (foreign_scope.to_string(), false),
        (foreign_revision.to_string(), false),
        ("malformed ready output".into(), false),
    ] {
        let mut responses = prefix()?;
        responses.extend([
            resolved(),
            ("470 Not Ready", String::new()),
            ("200 OK", body),
        ]);
        if accepted {
            responses.push(("200 OK", "null".into()));
        }
        let (captured, outcome) = exchange_mode(&[], responses).await?;
        check!(eq; captured.len(), if accepted { 7 } else { 6 });
        assert_lookup(&captured, 3)?;
        check!(captured.iter().skip(4).take(2).all(|(path, _)| path
            == &format!("GET /restate/invocation/{RESOLVED_ID}/output HTTP/1.1\r\n")));
        check!(eq; outcome.is_ok(), accepted);
        if let Err(error) = outcome {
            check!(
                format!("{error:#}").contains("national attachment"),
                "{error:#}"
            );
        }
    }
    Ok(())
}

#[tokio::test]
async fn managed_attached_observer_refuses_oversized_transport_reply_without_redriving(
) -> TestResult {
    let mut responses = prefix()?;
    responses.extend([
        resolved(),
        ("470 Not Ready", String::new()),
        ("470 Not Ready", String::new()),
    ]);
    let (captured, outcome) = exchange_with_held_reply(&[], responses, Some((5, 262_145))).await?;
    check!(eq; captured.len(), 6);
    assert_lookup(&captured, 3)?;
    check!(captured
        .iter()
        .skip(4)
        .all(|(path, _)| path
            == &format!("GET /restate/invocation/{RESOLVED_ID}/output HTTP/1.1\r\n")));
    let error = outcome.err().ok_or("oversized observed output accepted")?;
    let diagnostic = format!("{error:#}");
    check!(
        diagnostic.contains("national attachment output exceeded transport admission"),
        "{diagnostic}"
    );
    check!(diagnostic.contains("DeclaredBodyBytes"), "{diagnostic}");
    check!(diagnostic.contains(RESOLVED_ID), "{diagnostic}");
    Ok(())
}

#[test]
fn parser_size_admission_retains_transport_type_and_invocation_context() -> TestResult {
    let source = reqwest::Client::builder()
        .user_agent("\n")
        .build()
        .err()
        .ok_or("invalid header accepted")?;
    let source = census_crawl::ingress::IngressError::ParserSize {
        status: None,
        observed: None,
        invocation_id: Some(RESOLVED_ID.to_owned()),
        source,
    };
    let error =
        crate::cli::national::output::read_error(restate_sdk::ingress::ClientError::Transport {
            source: Box::new(source),
        });
    let client_error = error
        .downcast_ref::<restate_sdk::ingress::ClientError>()
        .ok_or("transport error type was lost")?;
    check!(census_crawl::ingress::is_admission_error(client_error));
    let diagnostic = format!("{error:#}");
    check!(diagnostic.contains(RESOLVED_ID));
    check!(diagnostic.contains("status=None"));
    let source = reqwest::Client::builder()
        .user_agent("\n")
        .build()
        .err()
        .ok_or("invalid header accepted")?;
    let error =
        crate::cli::national::output::read_error(restate_sdk::ingress::ClientError::Transport {
            source: Box::new(census_crawl::ingress::IngressError::Transport(source)),
        });
    let client_error = error
        .downcast_ref::<restate_sdk::ingress::ClientError>()
        .ok_or("transport error type was lost")?;
    check!(!census_crawl::ingress::is_admission_error(client_error));
    Ok(())
}
