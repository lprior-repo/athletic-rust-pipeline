use super::*;
use clap::Parser;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

type ReceiptTestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[tokio::test]
async fn national_report_reads_stored_publication_seal_without_submitting_a_run(
) -> ReceiptTestResult {
    let receipt = serde_json::json!({
        "phase": "sealed",
        "workbook": "out/publication/current/workbook.xlsx",
        "recorded": null,
        "sealed": {"digest": "controlled-seal-digest", "sealed_on": "2026-10-19"},
        "open": [],
        "refusal": null,
        "wrote": "out/seal.json",
        "counts": {
            "jurisdiction_buckets": 1,
            "schools": 2,
            "meets": 3,
            "athletes": 4,
            "class_of_2027": 5,
            "cohort_performances": 6,
            "coaches": 7
        },
        "retained": {
            "gaps": [],
            "conflicts": 0,
            "access_conditions": 0,
            "blocked_hosts": 0,
            "throttled_hosts": 0,
            "silent_sources": [],
            "source_failures": null,
            "observations": 8,
            "calculations": 9
        }
    });
    for response in [serde_json::to_string(&Some(receipt))?, "null".to_owned()] {
        let path = run_seal_report(&response).await?;
        check!(path.contains("/publication_seal "));
        check!(!path.contains("/run "));
    }
    Ok(())
}

#[test]
fn national_report_seal_and_contacts_modes_cannot_be_combined() -> ReceiptTestResult {
    let parsed = crate::cli::Cli::try_parse_from([
        "census-service",
        "national-report",
        "--seal",
        "--contacts",
    ]);
    check!(parsed.is_err());
    Ok(())
}

async fn run_seal_report(response_body: &str) -> ReceiptTestResult<String> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let origin = format!("http://{}/", listener.local_addr()?);
    let cli = crate::cli::Cli::try_parse_from([
        "census-service",
        "national-report",
        "--states",
        "WI",
        "--seal",
        "--ingress",
        origin.as_str(),
    ])?;
    let crate::cli::command::Command::NationalReport(args) = &cli.command else {
        return Err("fixture parsed another command".into());
    };
    let (path, outcome) = tokio::join!(
        respond_once(listener, response_body),
        super::run_national_report(&cli, args)
    );
    outcome?;
    path
}

async fn respond_once(
    listener: tokio::net::TcpListener,
    response_body: &str,
) -> ReceiptTestResult<String> {
    let (socket, _) = listener.accept().await?;
    let mut reader = BufReader::new(socket);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).await?;
    check!(!request_line.is_empty(), "request line missing");
    let mut body_len = 0_usize;
    for _ in 0..128 {
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        if line == "\r\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                body_len = value.trim().parse()?;
            }
        }
    }
    check!(body_len <= 8192, "read request body exceeds fixture limit");
    let mut request_body = vec![0; body_len];
    reader.read_exact(&mut request_body).await?;
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nx-restate-id: seal-readback-fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response_body.len(),
        response_body
    );
    reader.get_mut().write_all(response.as_bytes()).await?;
    reader.get_mut().shutdown().await?;
    Ok(request_line)
}
