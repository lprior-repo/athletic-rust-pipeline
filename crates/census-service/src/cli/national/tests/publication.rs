use super::resume::{bound_request, capabilities, guarded_resume_with_held_reply};
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
    exchange_mode(arguments, responses).await
}

async fn exchange_mode(
    arguments: &[&str],
    responses: Vec<(&str, String)>,
) -> TestResult<(Vec<(String, Vec<u8>)>, anyhow::Result<()>)> {
    exchange_with_held_reply(arguments, responses, None).await
}

async fn exchange_with_held_reply(
    arguments: &[&str],
    responses: Vec<(&str, String)>,
    held_reply: Option<(usize, usize)>,
) -> TestResult<(Vec<(String, Vec<u8>)>, anyhow::Result<()>)> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}/", listener.local_addr()?);
    let store = "a".repeat(64);
    let profile = "b".repeat(64);
    let mut input = vec!["census-service", "national", "--states", "WI"];
    input.extend(["--selected-store-identity", store.as_str()]);
    input.extend(["--selected-profile-id", profile.as_str()]);
    input.extend(["--generation", "qualified-v1"]);
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

fn previously_accepted() -> (&'static str, String) {
    (
        "202 Accepted",
        serde_json::json!({
            "invocationId": "inv_1aiqX0vFEFNH1Umgre58JiCLgHfTtztYK5", "status": "PreviouslyAccepted"
        })
        .to_string(),
    )
}

fn resolved() -> (&'static str, String) {
    (
        "200 OK",
        serde_json::json!({"invocationId": "inv_1aiqX0vFEFNH1Umgre58JiCLgHfTtztYK5"}).to_string(),
    )
}

fn ready_complete_report() -> TestResult<String> {
    Ok(serde_json::to_string(&serde_json::json!({
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

fn send_count(captured: &[(String, Vec<u8>)]) -> usize {
    captured
        .iter()
        .filter(|(path, _)| path.contains("/run/send "))
        .count()
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
    let (captured, _) =
        exercise(&[], vec![("200 OK", old_capabilities), absent(), absent()]).await?;
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
    let (captured, error) = exercise(
        &[],
        vec![
            ("200 OK", capabilities()?),
            absent(),
            ("200 OK", "{\"status\":\"unbound_legacy\"}".into()),
        ],
    )
    .await?;
    check!(eq; captured.len(), 3);
    check!(eq; send_count(&captured), 1);
    check!(captured
        .get(2)
        .is_some_and(|(path, _)| path.contains("/request HTTP/1.1")));
    check!(error.contains("invalid request binding"), "{error}");
    Ok(())
}

#[tokio::test]
async fn send_refusal_reuses_verified_retained_request_without_new_submission() -> TestResult {
    let (captured, outcome) = exchange(
        &[],
        vec![
            ("200 OK", capabilities()?),
            absent(),
            ("200 OK", bound_request(&["WI"])?),
            resolved(),
            ("200 OK", ready_complete_report()?),
            ("200 OK", ready_complete_report()?),
            ("200 OK", "null".into()),
        ],
    )
    .await?;
    outcome?;
    check!(eq; captured.len(), 7);
    check!(eq; send_count(&captured), 1);
    check!(captured
        .get(2)
        .is_some_and(|(path, _)| path.contains("/request HTTP/1.1")));
    check!(captured
        .get(3)
        .is_some_and(|(path, _)| path == "POST /restate/lookup HTTP/1.1\r\n"));
    check!(!captured
        .iter()
        .any(|(path, _)| path.contains("%3Aresume%3A")));
    Ok(())
}

#[tokio::test]
async fn incompatible_request_lookup_blocks_attachment_without_fallback_submission() -> TestResult {
    let (captured, error) =
        exercise(&[], vec![("200 OK", capabilities()?), absent(), absent()]).await?;
    check!(eq; captured.len(), 3);
    check!(eq; send_count(&captured), 1);
    check!(captured
        .get(2)
        .is_some_and(|(path, _)| path.contains("/request HTTP/1.1")));
    check!(error.contains("invalid request binding"), "{error}");
    Ok(())
}

#[tokio::test]
async fn pending_original_preflight_blocks_attachment_without_searching_other_keys() -> TestResult {
    let (captured, error) = exercise(
        &[],
        vec![
            ("200 OK", capabilities()?),
            previously_accepted(),
            ("200 OK", "{\"status\":\"pending_preflight\"}".into()),
        ],
    )
    .await?;
    check!(eq; captured.len(), 3);
    check!(eq; send_count(&captured), 1);
    check!(!captured
        .iter()
        .any(|(path, _)| path.contains("/output HTTP/1.1")));
    check!(
        error.contains("original school-address preflight is incomplete"),
        "{error}"
    );
    Ok(())
}

#[tokio::test]
async fn retained_request_requires_pinned_school_address_digest() -> TestResult {
    let mut pinned: serde_json::Value = serde_json::from_str(&bound_request(&["WI"])?)?;
    pinned
        .get_mut("request")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("fixture bound request")?
        .insert(
            "school_address".into(),
            serde_json::json!({
                "generation": "original-directory", "acquisitions": [],
                "expected_digest": "pinned-original-directory-digest"
            }),
        );
    let (captured, outcome) = exchange(
        &[],
        vec![
            ("200 OK", capabilities()?),
            absent(),
            ("200 OK", serde_json::to_string(&pinned)?),
            resolved(),
            ("200 OK", ready_complete_report()?),
            ("200 OK", ready_complete_report()?),
            ("200 OK", "null".into()),
        ],
    )
    .await?;
    outcome?;
    check!(eq; captured.len(), 7);
    check!(eq; send_count(&captured), 1);
    let mut unpinned = pinned.clone();
    unpinned
        .get_mut("request")
        .and_then(serde_json::Value::as_object_mut)
        .and_then(|request| request.get_mut("school_address"))
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("fixture school address")?
        .remove("expected_digest");
    let (captured, error) = exercise(
        &[],
        vec![
            ("200 OK", capabilities()?),
            absent(),
            ("200 OK", serde_json::to_string(&unpinned)?),
        ],
    )
    .await?;
    check!(eq; captured.len(), 3);
    check!(error.contains("no pinned directory digest"), "{error}");
    Ok(())
}
