use super::*;
use census_reconcile::identity::WorkflowIdentity;

async fn refused_resume_ingress(
    listener: tokio::net::TcpListener,
    responses: Vec<(&str, String)>,
) -> Result<Vec<(String, Vec<u8>)>, Box<dyn std::error::Error>> {
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
    let mut captured = Vec::new();
    for (status, body) in responses {
        let (socket, _) = listener.accept().await?;
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
        captured.push((path, payload));
        let response = format!(
            "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nX-Restate-Id: invocation-national-resume-fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        );
        socket.get_mut().write_all(response.as_bytes()).await?;
        socket.get_mut().shutdown().await?;
    }
    Ok(captured)
}

fn bound_history(as_of: Option<&str>) -> Result<String, Box<dyn std::error::Error>> {
    let state = census_service::restate_services::JurisdictionState {
        history_window: as_of
            .map(census_service::restate_services::HistoryWindow::cohort)
            .transpose()?,
        ..Default::default()
    };
    Ok(serde_json::to_string(&state)?)
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
        ("200 OK", bound_history(Some("2026-10-01"))?),
        refusal(),
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
            .is_some_and(|(path, _)| path.contains("/state HTTP/1.1")),
        "{captured:?}"
    );
    check!(
        captured
            .get(1)
            .is_some_and(|(path, _)| path.contains(&format!("/{expected}/run/send"))),
        "{captured:?}"
    );
    check!(
        captured
            .get(2)
            .is_some_and(|(path, _)| path.contains(&expected)),
        "{captured:?}"
    );
    let (_, payload) = captured.get(1).ok_or("resume request absent")?;
    let request: census_service::restate_services::NationalRequest =
        serde_json::from_slice(payload)?;
    check!(eq; request.revision, census_reconcile::identity::Revision(1));
    check!(eq; request.history, census_service::restate_services::HistoryWindow::cohort("2026-10-01")?);
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
    for (responses, expected) in cases {
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
                .all(|(path, _)| path.contains("/state HTTP/1.1")),
            "{captured:?}"
        );
    }
    Ok(())
}
