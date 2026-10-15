use super::*;
use census_reconcile::identity::WorkflowIdentity;

pub(super) async fn refused_resume_ingress(
    listener: tokio::net::TcpListener,
    responses: Vec<(&str, String)>,
) -> Result<Vec<(String, Vec<u8>)>, Box<dyn std::error::Error>> {
    let mut captured = Vec::new();
    for (status, body) in responses {
        let (socket, _) = listener.accept().await?;
        captured.push(record_and_respond(socket, status, &body).await?);
    }
    Ok(captured)
}

pub(super) async fn guarded_resume_ingress(
    listener: tokio::net::TcpListener,
    responses: Vec<(&str, String)>,
    completion: tokio::sync::oneshot::Receiver<()>,
) -> Result<Vec<(String, Vec<u8>)>, Box<dyn std::error::Error>> {
    guarded_resume_with_held_reply(listener, responses, completion, None).await
}

pub(super) async fn guarded_resume_with_held_reply(
    listener: tokio::net::TcpListener,
    responses: Vec<(&str, String)>,
    mut completion: tokio::sync::oneshot::Receiver<()>,
    held_reply: Option<(usize, usize)>,
) -> Result<Vec<(String, Vec<u8>)>, Box<dyn std::error::Error>> {
    let mut held_socket = None;
    let mut captured = Vec::new();
    let mut responses = responses.into_iter();
    loop {
        let socket = tokio::select! {
            biased;
            accepted = listener.accept() => accepted?.0,
            _ = &mut completion => { drop(held_socket); return Ok(captured); },
        };
        if captured.len() >= 1024 {
            return Err("resume fixture request count exceeds limit".into());
        }
        let (status, body) = match responses.next() {
            Some(response) => response,
            None => (
                "500 Internal Server Error",
                "{\"message\":\"unexpected request\"}".into(),
            ),
        };
        let declared = held_reply
            .filter(|(index, _)| *index == captured.len())
            .map(|(_, length)| length);
        let (record, held) = record_reply(socket, status, &body, declared).await?;
        if held.is_some() {
            held_socket = held;
        }
        captured.push(record);
    }
}

async fn record_and_respond(
    socket: tokio::net::TcpStream,
    status: &str,
    body: &str,
) -> Result<(String, Vec<u8>), Box<dyn std::error::Error>> {
    Ok(record_reply(socket, status, body, None).await?.0)
}

async fn record_reply(
    socket: tokio::net::TcpStream,
    status: &str,
    body: &str,
    held_length: Option<usize>,
) -> Result<((String, Vec<u8>), Option<tokio::net::TcpStream>), Box<dyn std::error::Error>> {
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    let mut socket = BufReader::new(socket);
    let mut path = String::new();
    socket.read_line(&mut path).await?;
    let mut length = 0;
    for _ in 0..128 {
        let mut header = String::new();
        socket.read_line(&mut header).await?;
        if header == "\r\n" {
            break;
        }
        if let Some((key, value)) = header.split_once(':') {
            if key.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse::<usize>()?;
            }
        }
    }
    if length > 8192 {
        return Err("resume fixture request exceeds byte ceiling".into());
    }
    let mut payload = vec![0; length];
    socket.read_exact(&mut payload).await?;
    let invocation_header = if status.starts_with("470 ") {
        ""
    } else {
        "X-Restate-Id: inv_1aiqX0vFEFNH1Umgre58JiCLgHfTtztYK5\r\n"
    };
    if held_length.is_some() && !body.is_empty() {
        return Err("held response must not provide a body".into());
    }
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\n{invocation_header}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        held_length.map_or(body.len(), |length| length)
    );
    socket.get_mut().write_all(response.as_bytes()).await?;
    if held_length.is_some() {
        return Ok(((path, payload), Some(socket.into_inner())));
    }
    socket.get_mut().shutdown().await?;
    Ok(((path, payload), None))
}

#[tokio::test]
async fn guarded_ingress_observes_unexpected_requests_even_when_errors_are_swallowed(
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}", listener.local_addr()?);
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()?;
    let (finished, completion) = tokio::sync::oneshot::channel::<()>();
    let (captured, outcome) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(
            guarded_resume_ingress(listener, vec![("200 OK", "{}".into())], completion),
            async {
                let first = client.post(format!("{origin}/expected")).send().await?;
                check!(eq; first.status(), reqwest::StatusCode::OK);
                let unexpected = client.post(format!("{origin}/unexpected")).send().await?;
                let swallowed_status = unexpected.status();
                drop(finished);
                Ok::<_, Box<dyn std::error::Error>>(swallowed_status)
            }
        )
    })
    .await?;
    let captured = captured?;
    check!(eq; outcome?, reqwest::StatusCode::INTERNAL_SERVER_ERROR);
    check!(eq; captured.len(), 2);
    check!(captured
        .get(1)
        .is_some_and(|(path, _)| path == "POST /unexpected HTTP/1.1\r\n"));
    Ok(())
}

pub(super) fn bound_history(as_of: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    let state = census_service::restate_services::JurisdictionState {
        history_window: as_of
            .map(census_service::restate_services::HistoryWindow::cohort)
            .transpose()?,
        ..Default::default()
    };
    Ok(serde_json::to_string(&state)?)
}

pub(super) fn bound_request(jurisdictions: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let request: census_service::restate_services::NationalRequest = serde_json::from_value(
        serde_json::json!({
            "season": 2026, "revision": 1, "jurisdictions": jurisdictions,
            "history": {"first_calendar_year": 2023, "last_calendar_year": 2026, "as_of": "2026-10-01"},
            "publication_contract": "managed_bundle_v2", "pass_budget": 17,
            "pass_delay_seconds": 41, "concurrency": 7
        }),
    )?;
    Ok(serde_json::to_string(
        &census_service::restate_services::NationalRequestBinding::Bound(Box::new(request)),
    )?)
}

pub(super) fn capabilities() -> Result<String, Box<dyn std::error::Error>> {
    Ok(serde_json::to_string(
        &census_service::restate_services::PublicationCapabilities::current(),
    )?)
}

fn refusal() -> (&'static str, String) {
    (
        "404 Not Found",
        "{\"message\":\"fixture invocation absent\"}".to_owned(),
    )
}

#[tokio::test]
async fn national_resume_submits_and_reattaches_the_selected_parent_not_a_new_revision(
) -> Result<(), Box<dyn std::error::Error>> {
    use clap::Parser;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}/", listener.local_addr()?);
    let cli = crate::cli::Cli::try_parse_from([
        "census-service",
        "national",
        "--states",
        "WI",
        "--revision",
        "1",
        "--resume",
        "7",
        "--ingress",
        &origin,
        "--detach",
    ])?;
    let crate::cli::command::Command::National(args) = &cli.command else {
        return Err("resume parsed as another command".into());
    };
    let responses = vec![
        ("200 OK", bound_request(&["WI"])?),
        ("200 OK", bound_history(Some("2026-10-01"))?),
        ("200 OK", capabilities()?),
        refusal(),
        ("200 OK", bound_request(&["WI"])?),
        refusal(),
    ];
    let (paths, outcome) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(
            refused_resume_ingress(listener, responses),
            run_national(&cli, args)
        )
    })
    .await?;
    let captured = paths?;
    check!(outcome.is_err());
    let expected = format!(
        "{}:resume:7",
        WorkflowIdentity::national(
            census_domain::model::SchoolYear::new(2026).ok_or("fixture season")?,
            census_reconcile::identity::Revision(1),
            &[census_domain::UsJurisdiction::Wisconsin],
        )
        .as_str()
    );
    let expected = expected.replace(':', "%3A");
    check!(
        captured
            .first()
            .is_some_and(|(path, _)| path.contains("/request HTTP/1.1")),
        "{captured:?}"
    );
    check!(
        captured
            .get(3)
            .is_some_and(|(path, _)| path.contains(&format!("/{expected}/run/send"))),
        "{captured:?}"
    );
    check!(
        captured
            .get(5)
            .is_some_and(|(path, body)| path == "POST /restate/lookup HTTP/1.1\r\n"
                && serde_json::from_slice::<serde_json::Value>(body)
                    .ok()
                    .and_then(|value| value
                        .get("workflowKey")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_owned))
                    .is_some_and(|key| key.replace(':', "%3A") == expected)),
        "{captured:?}"
    );
    check!(captured
        .get(1)
        .is_some_and(|(path, _)| path.contains("/state HTTP/1.1")));
    check!(captured
        .get(2)
        .is_some_and(|(path, _)| path.contains("/Census/publication_capabilities HTTP/1.1")));
    check!(captured
        .get(4)
        .is_some_and(|(path, _)| path.contains("/request HTTP/1.1")));
    let (_, payload) = captured.get(3).ok_or("resume request absent")?;
    let request: census_service::restate_services::NationalRequest =
        serde_json::from_slice(payload)?;
    check!(eq; request.revision, census_reconcile::identity::Revision(1));
    check!(eq; request.history, census_service::restate_services::HistoryWindow::cohort("2026-10-01")?);
    check!(eq; request.publication_contract, census_service::restate_services::PublicationContract::ManagedBundleV2);
    check!(eq; request.pass_budget, 17);
    check!(eq; request.pass_delay_seconds, 41);
    check!(eq; request.concurrency, 7);
    Ok(())
}

#[tokio::test]
async fn national_resume_report_reads_the_selected_execution(
) -> Result<(), Box<dyn std::error::Error>> {
    use clap::Parser;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}/", listener.local_addr()?);
    let cli = crate::cli::Cli::try_parse_from([
        "census-service",
        "national-report",
        "--states",
        "WI",
        "--revision",
        "1",
        "--resume",
        "7",
        "--ingress",
        &origin,
    ])?;
    let crate::cli::command::Command::NationalReport(args) = &cli.command else {
        return Err("resume report parsed as another command".into());
    };
    let (paths, outcome) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(
            refused_resume_ingress(listener, vec![refusal()]),
            run_national_report(&cli, args)
        )
    })
    .await?;
    let paths = paths?.into_iter().map(|(path, _)| path).collect::<Vec<_>>();
    check!(outcome.is_err());
    let identity = WorkflowIdentity::national(
        census_domain::model::SchoolYear::new(2026).ok_or("fixture season")?,
        census_reconcile::identity::Revision(1),
        &[census_domain::UsJurisdiction::Wisconsin],
    );
    let expected = format!("{}:resume:7", identity.as_str()).replace(':', "%3A");
    check!(eq; paths, vec![format!("POST /NationalCensus/{expected}/report HTTP/1.1\r\n")]);
    Ok(())
}

#[test]
fn national_resume_rejects_zero_negative_and_overflow_ordinals(
) -> Result<(), Box<dyn std::error::Error>> {
    use clap::Parser;
    for command in ["national", "national-report"] {
        for ordinal in ["0", "-1", "4294967296"] {
            let flag = format!("--resume={ordinal}");
            let error = crate::cli::Cli::try_parse_from(["census-service", command, flag.as_str()])
                .err()
                .ok_or("invalid resume ordinal admitted")?;
            check!(eq; error.kind(), clap::error::ErrorKind::ValueValidation);
        }
    }
    Ok(())
}

#[tokio::test]
async fn national_resume_refuses_unbound_conflicting_and_unreadable_history_before_submission(
) -> Result<(), Box<dyn std::error::Error>> {
    use clap::Parser;
    let cases = [
        (
            vec![
                ("200 OK", bound_history(None)?),
                ("200 OK", bound_history(None)?),
            ],
            "no jurisdiction",
        ),
        (
            vec![
                ("200 OK", bound_history(Some("2026-10-01"))?),
                ("200 OK", bound_history(Some("2026-10-02"))?),
            ],
            "disagrees",
        ),
        (
            vec![(
                "500 Internal Server Error",
                "{\"message\":\"retained state unreadable\"}".to_owned(),
            )],
            "retained state unreadable",
        ),
    ];
    for (mut responses, expected) in cases {
        responses.insert(0, ("200 OK", bound_request(&["WI", "IA"])?));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let origin = format!("http://{}/", listener.local_addr()?);
        let cli = crate::cli::Cli::try_parse_from([
            "census-service",
            "national",
            "--states",
            "WI,IA",
            "--resume",
            "7",
            "--ingress",
            &origin,
            "--detach",
        ])?;
        let crate::cli::command::Command::National(args) = &cli.command else {
            return Err("resume parsed as another command".into());
        };
        let (captured, outcome) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            tokio::join!(
                refused_resume_ingress(listener, responses),
                run_national(&cli, args)
            )
        })
        .await?;
        let captured = captured?;
        let error = outcome.err().ok_or("invalid retained history admitted")?;
        check!(error.to_string().contains(expected), "{error}");
        check!(
            captured
                .iter()
                .all(|(path, _)| path.contains("/request HTTP/1.1")
                    || path.contains("/state HTTP/1.1")),
            "{captured:?}"
        );
    }
    Ok(())
}
