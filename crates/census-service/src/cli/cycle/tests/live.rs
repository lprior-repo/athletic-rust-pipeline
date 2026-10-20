use super::*;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

type Observations = Vec<(String, Vec<u8>)>;

async fn exchange(
    arguments: &[&str],
    responses: Vec<(&str, String)>,
) -> TestResult<(Observations, anyhow::Result<()>)> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}/", listener.local_addr()?);
    let store_identity = "a".repeat(64);
    let profile_id = "b".repeat(64);
    let input = ["census-service", "run", "--school-year", "2026"]
        .into_iter()
        .chain([
            "--selected-store-identity",
            store_identity.as_str(),
            "--selected-profile-id",
            profile_id.as_str(),
        ])
        .chain(arguments.iter().copied())
        .chain(["--ingress", &origin]);
    let cli = Cli::try_parse_from(input)?;
    let Command::Run(args) = &cli.command else {
        return Err("fixture did not parse standalone run".into());
    };
    let (finished, completion) = oneshot::channel::<()>();
    let (requests, outcome) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(respond(listener, responses, completion), async {
            let outcome = run_cycle(&cli, args).await;
            drop(finished);
            outcome
        })
    })
    .await?;
    Ok((requests?, outcome))
}

async fn respond(
    listener: TcpListener,
    responses: Vec<(&str, String)>,
    mut completion: oneshot::Receiver<()>,
) -> TestResult<Observations> {
    let mut requests = Vec::new();
    let mut responses = responses.into_iter();
    loop {
        let stream = tokio::select! {
            biased;
            accepted = listener.accept() => accepted?.0,
            _ = &mut completion => return Ok(requests),
        };
        if requests.len() >= 8 {
            return Err("fixture request count exceeds limit".into());
        }
        let (status, body) = match responses.next() {
            Some(response) => response,
            None => (
                "500 Internal Server Error",
                "{\"message\":\"unexpected request\"}".into(),
            ),
        };
        requests.push(record_and_respond(stream, status, &body).await?);
    }
}

async fn record_and_respond(
    stream: tokio::net::TcpStream,
    status: &str,
    body: &str,
) -> TestResult<(String, Vec<u8>)> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    let mut length = 0;
    for _ in 0..64 {
        let mut header = String::new();
        reader.read_line(&mut header).await?;
        if header == "\r\n" {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse::<usize>()?;
            }
        }
    }
    if length > 8192 {
        return Err("fixture request body exceeds byte limit".into());
    }
    let mut payload = vec![0; length];
    reader.read_exact(&mut payload).await?;
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nX-Restate-Id: invocation-managed-cycle-fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()
    );
    reader.get_mut().write_all(response.as_bytes()).await?;
    reader.get_mut().shutdown().await?;
    Ok((line, payload))
}

fn capabilities() -> TestResult<String> {
    Ok(serde_json::to_string(&PublicationCapabilities::current())?)
}

fn workbook_reply(grad_year: i16) -> (&'static str, String) {
    (
        "200 OK",
        serde_json::json!({
            "path": "out/publication/generations/fixture/workbook.xlsx",
            "grad_year": grad_year,
        })
        .to_string(),
    )
}

#[tokio::test]
async fn selected_live_run_preserves_selection_destination_and_generation() -> TestResult {
    let (requests, outcome) = exchange(
        &[
            "--grad-year",
            "2028",
            "--core",
            "--limit",
            "7",
            "--out",
            "selected-publication",
            "--generation",
            "managed-2",
        ],
        vec![("200 OK", capabilities()?), workbook_reply(2028)],
    )
    .await?;
    outcome?;
    check!(eq; requests.len(), 2);
    let (line, payload) = requests.get(1).ok_or("selected workbook request missing")?;
    let request: WorkbookRequest = serde_json::from_slice(payload)?;
    check!(eq; request.grad_year, Some(2028));
    check!(eq; request.scope.as_deref(), Some("core"));
    check!(eq; request.limit, Some(7));
    check!(eq; request.out.as_deref(), Some("selected-publication"));
    let key = workbook_request_key(&request, &ExportGeneration::parse("managed-2")?)?;
    check!(eq; line, &format!("POST /Workbook/{}/run HTTP/1.1\r\n", key.replace(':', "%3A")));
    Ok(())
}

#[tokio::test]
async fn unsupported_or_malformed_capability_blocks_all_publication_calls() -> TestResult {
    for (response, diagnostic) in [
        (
            ("404 Not Found", "{\"message\":\"not supported\"}".into()),
            "publication capability response is invalid; no invocation was submitted",
        ),
        (
            ("200 OK", "{}".into()),
            "publication capability response is invalid; no invocation was submitted",
        ),
        (
            (
                "200 OK",
                "{\"publication_contracts\":[\"legacy_separate_v1\"]}".into(),
            ),
            "endpoint does not advertise SelectedPublicationV1; no invocation was submitted",
        ),
        (
            ("200 OK", "{\"publication_contracts\":[\"unknown\"]}".into()),
            "publication capability response is invalid; no invocation was submitted",
        ),
        (
            ("200 OK", "not json".into()),
            "publication capability response is invalid; no invocation was submitted",
        ),
    ] {
        let (requests, outcome) =
            exchange(&["--generation", "qualified-v1"], vec![response]).await?;
        let error = outcome.err().ok_or("unsupported endpoint accepted")?;
        check!(eq; requests.len(), 1);
        check!(requests
            .first()
            .is_some_and(|(line, _)| line.contains("/Census/publication_capabilities HTTP/1.1")));
        check!(format!("{error:#}").starts_with(diagnostic), "{error:#}");
    }
    Ok(())
}

#[tokio::test]
async fn failed_selected_workbook_never_falls_back_to_separate_exports() -> TestResult {
    let (requests, outcome) = exchange(
        &["--generation", "qualified-v1"],
        vec![
            ("200 OK", capabilities()?),
            (
                "409 Conflict",
                "{\"message\":\"fixture publication refused\"}".into(),
            ),
        ],
    )
    .await?;
    let error = outcome
        .err()
        .ok_or("refused workbook unexpectedly succeeded")?;
    check!(eq; requests.len(), 2);
    check!(requests
        .get(1)
        .is_some_and(|(line, _)| line.starts_with("POST /Workbook/")));
    check!(format!("{error:#}").contains("fixture publication refused"));
    Ok(())
}

async fn reject_live_selection(arguments: &[&str]) -> TestResult {
    let mut input = vec![
        "census-service".to_string(),
        "run".to_string(),
        "--school-year".to_string(),
        "2026".to_string(),
        "--ingress".to_string(),
        "http://127.0.0.1:1/".to_string(),
    ];
    input.extend(arguments.iter().map(|argument| (*argument).to_owned()));
    let cli = Cli::try_parse_from(input)?;
    let Command::Run(args) = &cli.command else {
        return Err("fixture did not parse standalone run".into());
    };
    run_cycle(&cli, args)
        .await
        .err()
        .ok_or("live cycle accepted incomplete selection")?;
    Ok(())
}

#[tokio::test]
async fn live_cycle_refuses_selection_before_ingress_acquisition() -> TestResult {
    reject_live_selection(&[]).await?;
    let store = "a".repeat(64);
    reject_live_selection(&["--selected-store-identity", store.as_str()]).await?;
    reject_live_selection(&["--selected-store-identity", "not-a-store-id"]).await?;
    Ok(())
}
