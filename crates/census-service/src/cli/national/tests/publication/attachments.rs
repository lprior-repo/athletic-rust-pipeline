use super::*;

#[path = "attachments/lookup.rs"]
mod lookup;

const RESOLVED_ID: &str = "inv_1aiqX0vFEFNH1Umgre58JiCLgHfTtztYK5";

fn output_path(path: &str) -> bool {
    path.starts_with("GET /restate/invocation/") && path.ends_with("/output HTTP/1.1\r\n")
}

fn invalid_bindings() -> TestResult<Vec<String>> {
    let selected: serde_json::Value = serde_json::from_str(&bound_request(&["WI"])?)?;
    let mut foreign = selected.clone();
    let season = foreign
        .get_mut("request")
        .and_then(|request| request.get_mut("season"))
        .ok_or("fixture season")?;
    *season = serde_json::json!(2025);
    let mut legacy = selected.clone();
    legacy
        .get_mut("request")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("fixture bound request")?
        .insert(
            "publication_contract".into(),
            serde_json::json!("legacy_separate_v1"),
        );
    let mut managed = selected.clone();
    managed
        .get_mut("request")
        .and_then(|request| request.as_object_mut())
        .ok_or("fixture bound request")?
        .insert(
            "publication_contract".into(),
            serde_json::json!("managed_bundle_v2"),
        );
    Ok(vec![
        "{\"status\":\"unbound_legacy\"}".into(),
        "{\"status\":\"pending_preflight\"}".into(),
        serde_json::to_string(&legacy)?,
        serde_json::to_string(&managed)?,
        serde_json::to_string(&foreign)?,
    ])
}

#[tokio::test]
async fn selected_previously_accepted_refuses_nonselected_unbound_pending_or_foreign_invocations(
) -> TestResult {
    for binding in invalid_bindings()? {
        let (captured, _) = exercise(
            &[],
            vec![
                ("200 OK", capabilities()?),
                previously_accepted(),
                ("200 OK", binding),
            ],
        )
        .await?;
        check!(eq; captured.len(), 3);
        check!(captured
            .get(2)
            .is_some_and(|(path, _)| path.contains("/request HTTP/1.1")));
        check!(!captured.iter().any(|(path, _)| output_path(path)));
    }
    Ok(())
}

#[tokio::test]
async fn selected_send_error_refuses_nonselected_unbound_pending_or_foreign_attachments(
) -> TestResult {
    for binding in invalid_bindings()? {
        let (captured, _) = exercise(
            &[],
            vec![("200 OK", capabilities()?), absent(), ("200 OK", binding)],
        )
        .await?;
        check!(eq; captured.len(), 3);
        check!(captured
            .get(2)
            .is_some_and(|(path, _)| path.contains("/request HTTP/1.1")));
        check!(!captured.iter().any(|(path, _)| output_path(path)));
    }
    Ok(())
}

#[tokio::test]
async fn selected_previously_accepted_uses_verified_retained_request() -> TestResult {
    let (captured, outcome) = exchange(
        &[],
        vec![
            ("200 OK", capabilities()?),
            previously_accepted(),
            ("200 OK", bound_request(&["WI"])?),
            ("200 OK", ready_complete_report()?),
            ("200 OK", "null".into()),
        ],
    )
    .await?;
    outcome?;
    check!(eq; captured.len(), 5);
    check!(captured
        .get(2)
        .is_some_and(|(path, _)| path.contains("/request HTTP/1.1")));
    let (_, submitted) = captured.get(1).ok_or("submission absent")?;
    check!(eq; serde_json::from_slice::<NationalRequest>(submitted)?.pass_budget, census_service::restate_services::default_pass_budget());
    Ok(())
}

#[tokio::test]
async fn selected_send_error_verifies_retained_request_before_attaching() -> TestResult {
    let (captured, outcome) = exchange(
        &[],
        vec![
            ("200 OK", capabilities()?),
            absent(),
            ("200 OK", bound_request(&["WI"])?),
            resolved(),
            ("470 Not Ready", String::new()),
            ("200 OK", ready_complete_report()?),
            ("200 OK", "null".into()),
        ],
    )
    .await?;
    outcome?;
    check!(eq; captured.len(), 7);
    check!(captured
        .get(2)
        .is_some_and(|(path, _)| path.contains("/request HTTP/1.1")));
    lookup::assert_lookup(&captured, 3)?;
    check!(captured
        .get(4)
        .is_some_and(|(path, _)| path
            == &format!("GET /restate/invocation/{RESOLVED_ID}/output HTTP/1.1\r\n")));
    check!(captured
        .last()
        .is_some_and(|(path, _)| path.contains("/contact_report ")));
    Ok(())
}

#[test]
fn nonselected_submission_refuses_before_any_ingress_request() -> TestResult {
    let encoded: serde_json::Value = serde_json::from_str(&bound_request(&["WI"])?)?;
    let mut request = encoded.get("request").ok_or("bound fixture")?.clone();
    request
        .as_object_mut()
        .ok_or("bound fixture request")?
        .insert(
            "publication_contract".into(),
            serde_json::json!("legacy_separate_v1"),
        );
    check!(serde_json::from_value::<NationalRequest>(request).is_err());
    Ok(())
}
