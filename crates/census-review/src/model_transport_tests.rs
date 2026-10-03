use std::time::Duration;

use census_domain::model::VerdictBatch;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::{ModelClient, ModelError, ModelOptions};
use crate::consensus::tests::support::TestResult;

#[path = "model_transport_tests/response_format.rs"]
mod response_format;

enum Reply {
    Complete(u16, &'static str),
    TextOnly(String),
    Redirect(u16, String),
    StalledHeaders,
    StalledBody,
    ChunkOverflow,
}

async fn serve(stream: TcpStream, reply: Reply) -> TestResult {
    let (stream, request) = read_request(stream).await?;
    send_reply(stream, reply, request).await
}

async fn read_request(
    stream: TcpStream,
) -> TestResult<(tokio::io::BufReader<TcpStream>, serde_json::Value)> {
    let mut stream = tokio::io::BufReader::new(stream);
    let mut length = 0;
    let mut content_type = None;
    loop {
        let mut line = String::new();
        check!(ne; stream.read_line(&mut line).await?, 0);
        if line == "\r\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim().to_ascii_lowercase();
            if name == "content-length" {
                length = value.trim().parse::<usize>()?;
            }
            if name == "content-type" {
                content_type = Some(value.trim().to_string());
            }
        }
    }
    check!(eq; content_type.as_deref(), Some("application/json"));
    check!(length <= super::REQUEST_CAP);
    let mut body = vec![0; length];
    stream.read_exact(&mut body).await?;
    let request = serde_json::from_slice(&body)?;
    Ok((stream, request))
}

async fn send_reply(
    mut stream: tokio::io::BufReader<TcpStream>,
    reply: Reply,
    request: serde_json::Value,
) -> TestResult {
    match reply {
        Reply::StalledHeaders => std::future::pending::<()>().await,
        Reply::Complete(status, body_text) => {
            send_complete(&mut stream, status, body_text).await?;
        }
        Reply::TextOnly(content) => {
            let (status, body) = if request["response_format"]["type"] == "text" {
                (
                    200,
                    serde_json::json!({
                        "choices": [{"message": {"content": content}}]
                    }),
                )
            } else {
                (
                    400,
                    serde_json::json!({
                        "error": {"code": "response_format_not_supported",
                            "message": "only response_format type=text is supported"}
                    }),
                )
            };
            send_complete(&mut stream, status, &body.to_string()).await?;
        }
        Reply::Redirect(status, location) => {
            let headers = format!(
                "HTTP/1.1 {status} Redirect\r\nLocation: {location}\r\nConnection: close\r\n\r\n"
            );
            stream.write_all(headers.as_bytes()).await?;
        }
        Reply::StalledBody => {
            let headers =
                "HTTP/1.1 200 Fixture\r\nContent-Length: 1000\r\nConnection: close\r\n\r\n";
            stream.write_all(headers.as_bytes()).await?;
            std::future::pending::<()>().await;
        }
        Reply::ChunkOverflow => {
            let headers =
                "HTTP/1.1 200 Fixture\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
            stream.write_all(headers.as_bytes()).await?;
            let chunk = "X".repeat(300_000);
            let chunk_line = format!("{:x}\r\n{}\r\n", chunk.len(), chunk);
            stream.write_all(chunk_line.as_bytes()).await?;
            stream.write_all(b"0\r\n\r\n").await?;
        }
    }
    stream.shutdown().await?;
    Ok(())
}

async fn send_complete(
    stream: &mut tokio::io::BufReader<TcpStream>,
    status: u16,
    body: &str,
) -> TestResult {
    let headers = format!(
        "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes()).await?;
    stream.write_all(body.as_bytes()).await?;
    Ok(())
}

async fn request(reply: Reply) -> TestResult<Result<VerdictBatch, ModelError>> {
    let stalled = matches!(reply, Reply::StalledHeaders | Reply::StalledBody);
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(5), async move {
            let (stream, _) = listener.accept().await?;
            serve(stream, reply).await
        })
        .await?
    });
    let mut options = ModelOptions::local(&format!("http://{address}"), "fixture-model")?;
    options = options.with_timeout(if stalled {
        Duration::from_millis(100)
    } else {
        Duration::from_secs(2)
    });
    let client = ModelClient::new(options)?;
    let packet = census_domain::model::ReviewPacket::new("school:test", "Test");
    let result = tokio::time::timeout(Duration::from_secs(5), client.adjudicate(&packet)).await;
    if stalled {
        server.abort();
        match server.await {
            Err(error) => check!(error.is_cancelled()),
            Ok(_) => return Err("HTTP fixture completed instead of being cancelled".into()),
        }
    } else {
        server.await??;
    }
    Ok(result?)
}

#[test]
fn http_503_returns_status_error() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            match request(Reply::Complete(503, "overloaded")).await? {
                Err(ModelError::Status { status }) => check!(eq; status, 503),
                other => return Err(format!("expected HTTP status failure, got {other:?}").into()),
            }
            Ok(())
        })
}

#[test]
fn malformed_batch_returns_content_error() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let body =
                r#"{"choices":[{"message":{"content":"PRIVATE_MODEL_SENTINEL_invalid_json"}}]}"#;
            let error = match request(Reply::Complete(200, body)).await? {
                Err(error) => error,
                Ok(_) => return Err("expected malformed model batch".into()),
            };
            check!(matches!(error, ModelError::Content { .. }));
            check!(!error.to_string().contains("PRIVATE_MODEL_SENTINEL"));
            Ok(())
        })
}

#[test]
fn empty_message_returns_empty_error() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let body = r#"{"choices":[{"message":{"content":""}}]}"#;
            check!(matches!(
                request(Reply::Complete(200, body)).await?,
                Err(ModelError::Empty)
            ));
            Ok(())
        })
}

#[test]
fn header_stall_returns_timeout() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            match request(Reply::StalledHeaders).await? {
                Err(ModelError::RequestTimeout) => {}
                other => return Err(format!("expected timeout, got {other:?}").into()),
            }
            Ok(())
        })
}

#[test]
fn body_stall_returns_timeout() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            match request(Reply::StalledBody).await? {
                Err(ModelError::RequestTimeout) => {}
                other => return Err(format!("expected timeout, got {other:?}").into()),
            }
            Ok(())
        })
}

#[test]
fn redirect_is_not_followed() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let listener = TcpListener::bind("127.0.0.1:0").await?;
            let address = listener.local_addr()?;
            check!(matches!(
                request(Reply::Redirect(302, format!("http://{address}"))).await?,
                Err(ModelError::Status { status: 302 })
            ));
            check!(
                tokio::time::timeout(Duration::from_millis(50), listener.accept())
                    .await
                    .is_err()
            );
            Ok(())
        })
}
#[test]
fn chunked_overflow_returns_response_error() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let result = request(Reply::ChunkOverflow).await?;
            match result {
                Err(ModelError::ResponseTooLarge { bytes }) => check!(bytes > 256_000),
                other => return Err(format!("expected response overflow, got {other:?}").into()),
            }
            Ok(())
        })
}
