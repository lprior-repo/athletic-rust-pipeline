use super::resume::{
    bound_history, bound_request, capabilities, guarded_resume_ingress,
    guarded_resume_with_held_reply,
};
use census_service::restate_services::{NationalRequest, PublicationContract};
use clap::Parser;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[path = "publication/attachments.rs"]
mod attachments;

#[path = "publication/drive.rs"]
mod drive;

async fn exercise(
    arguments: &[&str],
    responses: Vec<(&str, String)>,
) -> TestResult<(Vec<(String, Vec<u8>)>, String)> {
    let (captured, outcome) = exchange(arguments, responses).await?;
    let error = outcome
        .err()
        .ok_or("fixture submission unexpectedly succeeded")?;
    Ok((captured, format!("{error:#}")))
}

async fn exchange(
    arguments: &[&str],
    responses: Vec<(&str, String)>,
) -> TestResult<(Vec<(String, Vec<u8>)>, anyhow::Result<()>)> {
    exchange_mode(arguments, responses, true).await
}

async fn exchange_mode(
    arguments: &[&str],
    responses: Vec<(&str, String)>,
    detached: bool,
) -> TestResult<(Vec<(String, Vec<u8>)>, anyhow::Result<()>)> {
    exchange_with_held_reply(arguments, responses, detached, None).await
}

async fn exchange_with_held_reply(
    arguments: &[&str],
    responses: Vec<(&str, String)>,
    detached: bool,
    held_reply: Option<(usize, usize)>,
) -> TestResult<(Vec<(String, Vec<u8>)>, anyhow::Result<()>)> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}/", listener.local_addr()?);
    let store = "a".repeat(64);
    let profile = "b".repeat(64);
    let mut input = vec!["census-service", "national", "--states", "WI"];
    if detached {
        input.push("--detach");
    }
    input.extend(["--selected-store-identity", store.as_str()]);
    input.extend(["--selected-profile-id", profile.as_str()]);
    input.extend(arguments);
    input.extend(["--ingress", &origin]);
    let cli = crate::cli::Cli::try_parse_from(input)?;
    let crate::cli::command::Command::National(args) = &cli.command else {
        return Err("fixture parsed another command".into());
    };
    let (finished, completion) = tokio::sync::oneshot::channel::<()>();
    let (captured, outcome) = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        tokio::join!(
            guarded_resume_with_held_reply(listener, responses, completion, held_reply),
            async {
                let outcome = super::run_national(&cli, args).await;
                drop(finished);
                outcome
            }
        )
    })
    .await?;
    Ok((captured?, outcome))
}

fn absent() -> (&'static str, String) {
    (
        "404 Not Found",
        "{\"message\":\"fixture invocation absent\"}".into(),
    )
}

#[tokio::test]
async fn new_national_submission_checks_capability_before_any_write() -> TestResult {
    let (captured, _) =
        exercise(&[], vec![("200 OK", capabilities()?), absent(), absent()]).await?;
    check!(eq; captured.len(), 3);
    check!(captured
        .first()
        .is_some_and(|(path, _)| path == "POST /Census/publication_capabilities HTTP/1.1\r\n"));
    let (path, body) = captured.get(1).ok_or("submission absent")?;
    check!(path.contains("/run/send HTTP/1.1"));
    let request: NationalRequest = serde_json::from_slice(body)?;
    check!(eq; request.publication_contract, PublicationContract::SelectedPublicationV2);
    check!(matches!(
        request.export_job,
        census_report::export::ExportJobRequest::SelectedPublicationV1 { .. }
    ));
    Ok(())
}

#[tokio::test]
async fn selected_only_endpoint_cannot_receive_generation_bound_national_submission() -> TestResult
{
    let old_capabilities = serde_json::json!({
        "publication_contracts": ["selected_publication_v1"]
    })
    .to_string();
    let (captured, _) = exercise(
        &["--generation", "qualified-v1"],
        vec![("200 OK", old_capabilities), absent(), absent()],
    )
    .await?;
    check!(eq; captured.len(), 1);
    check!(captured
        .first()
        .is_some_and(|(path, _)| { path == "POST /Census/publication_capabilities HTTP/1.1\r\n" }));
    Ok(())
}

#[tokio::test]
async fn fresh_submission_without_complete_selection_refuses_before_any_network() -> TestResult {
    use clap::Parser;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}/", listener.local_addr()?);
    let cli = crate::cli::Cli::try_parse_from([
        "census-service",
        "national",
        "--states",
        "WI",
        "--detach",
        "--ingress",
        origin.as_str(),
    ])?;
    let crate::cli::command::Command::National(args) = &cli.command else {
        return Err("fixture parsed another command".into());
    };
    let (finished, completion) = tokio::sync::oneshot::channel::<()>();
    let (captured, outcome) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(
            super::resume::guarded_resume_ingress(listener, vec![], completion),
            async {
                let outcome = super::run_national(&cli, args).await;
                drop(finished);
                outcome
            }
        )
    })
    .await?;
    let captured = captured?;
    check!(outcome.is_err());
    check!(eq; captured.len(), 0);
    Ok(())
}

#[tokio::test]
async fn unsupported_missing_or_malformed_capability_never_submits_a_national_run() -> TestResult {
    for response in [
        absent(),
        ("200 OK", "{}".into()),
        (
            "200 OK",
            "{\"publication_contracts\":[\"legacy_separate_v1\"]}".into(),
        ),
        (
            "200 OK",
            "{\"publication_contracts\":[\"future_contract\"]}".into(),
        ),
        ("200 OK", "not json".into()),
    ] {
        let (captured, _) = exercise(&[], vec![response]).await?;
        check!(eq; captured.len(), 1);
        check!(captured
            .first()
            .is_some_and(|(path, _)| path.contains("/Census/publication_capabilities HTTP/1.1")));
    }
    Ok(())
}

#[tokio::test]
async fn unbound_legacy_resume_does_not_invent_a_contract_or_submission() -> TestResult {
    let unbound = "{\"status\":\"unbound_legacy\"}".to_owned();
    let (captured, error) = exercise(
        &["--resume", "7"],
        vec![
            ("200 OK", unbound.clone()),
            ("200 OK", unbound.clone()),
            ("200 OK", unbound),
        ],
    )
    .await?;
    check!(eq; captured.len(), 3);
    check!(captured
        .iter()
        .all(|(path, _)| path.contains("/request HTTP/1.1")));
    check!(captured
        .first()
        .is_some_and(|(path, _)| path.contains("%3Aresume%3A7/request")));
    check!(captured
        .get(1)
        .is_some_and(|(path, _)| !path.contains("%3Aresume%3A")));
    check!(captured
        .get(2)
        .is_some_and(|(path, _)| path.contains("%3Aresume%3A6/request")));
    check!(
        error.contains("no bound selected publication request"),
        "{error}"
    );
    Ok(())
}

#[tokio::test]
async fn new_resume_reuses_base_request_and_preserves_nondefault_budgets() -> TestResult {
    let original = bound_request(&["WI"])?;
    let (captured, _) = exercise(
        &["--resume", "7", "--pass-budget", "999"],
        vec![
            ("200 OK", "{\"status\":\"unbound_legacy\"}".into()),
            ("200 OK", original.clone()),
            ("200 OK", bound_history(Some("2026-10-01"))?),
            ("200 OK", capabilities()?),
            absent(),
            absent(),
        ],
    )
    .await?;
    check!(eq; captured.len(), 6);
    let (path, body) = captured.get(4).ok_or("resume submission absent")?;
    check!(path.contains("%3Aresume%3A7/run/send"));
    let binding: serde_json::Value = serde_json::from_str(&original)?;
    check!(eq; serde_json::from_slice::<serde_json::Value>(body)?, *binding.get("request").ok_or("bound request absent")?);
    Ok(())
}

#[tokio::test]
async fn incompatible_request_lookup_blocks_resume_without_fallback_submission() -> TestResult {
    let (captured, error) = exercise(&["--resume", "7"], vec![absent()]).await?;
    check!(eq; captured.len(), 1);
    check!(captured
        .first()
        .is_some_and(|(path, _)| path.contains("/request HTTP/1.1")));
    check!(
        error.contains("national resume compatibility blocked"),
        "{error}"
    );
    Ok(())
}

#[tokio::test]
async fn pending_original_preflight_blocks_resume_without_searching_other_keys() -> TestResult {
    let (captured, error) = exercise(
        &["--resume", "7"],
        vec![("200 OK", "{\"status\":\"pending_preflight\"}".into())],
    )
    .await?;
    check!(eq; captured.len(), 1);
    check!(captured
        .first()
        .is_some_and(|(path, _)| path.contains("%3Aresume%3A7/request")));
    check!(
        error.contains("original school-address preflight is incomplete"),
        "{error}"
    );
    Ok(())
}

#[tokio::test]
async fn resume_submission_preserves_the_bound_school_address_digest() -> TestResult {
    let mut original: serde_json::Value = serde_json::from_str(&bound_request(&["WI"])?)?;
    original
        .get_mut("request")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("fixture bound request")?
        .insert(
            "school_address".into(),
            serde_json::json!({
                "generation": "original-directory", "urls": {}, "dates": {},
                "expected_digest": "pinned-original-directory-digest"
            }),
        );
    let (captured, _) = exercise(
        &["--resume", "7"],
        vec![
            ("200 OK", serde_json::to_string(&original)?),
            ("200 OK", bound_history(Some("2026-10-01"))?),
            ("200 OK", capabilities()?),
            absent(),
            absent(),
        ],
    )
    .await?;
    check!(eq; captured.len(), 5);
    let (_, body) = captured.get(3).ok_or("resume submission absent")?;
    check!(eq; serde_json::from_slice::<serde_json::Value>(body)?, *original.get("request").ok_or("fixture bound request")?);
    Ok(())
}
