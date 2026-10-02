use std::time::Duration;

use census_domain::model::VerdictBatch;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use super::{ModelClient, ModelError, ModelOptions};

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

async fn serve(stream: TcpStream, reply: Reply) {
    let (stream, request) = read_request(stream).await;
    send_reply(stream, reply, request).await;
}

async fn read_request(stream: TcpStream) -> (tokio::io::BufReader<TcpStream>, serde_json::Value) {
    let mut stream = tokio::io::BufReader::new(stream);
    let mut length = 0;
    let mut content_type = None;
    loop {
        let mut line = String::new();
        assert_ne!(stream.read_line(&mut line).await.unwrap(), 0);
        if line == "\r\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim().to_ascii_lowercase();
            if name == "content-length" {
                length = value.trim().parse::<usize>().unwrap();
            }
            if name == "content-type" {
                content_type = Some(value.trim().to_string());
            }
        }
    }
    assert_eq!(content_type.as_deref(), Some("application/json"));
    assert!(length <= super::REQUEST_CAP);
    let mut body = vec![0; length];
    stream.read_exact(&mut body).await.unwrap();
    let request = serde_json::from_slice(&body).expect("request JSON");
    (stream, request)
}

async fn send_reply(
    mut stream: tokio::io::BufReader<TcpStream>,
    reply: Reply,
    request: serde_json::Value,
) {
    match reply {
        Reply::StalledHeaders => std::future::pending::<()>().await,
        Reply::Complete(status, body_text) => {
            send_complete(&mut stream, status, body_text).await;
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
            send_complete(&mut stream, status, &body.to_string()).await;
        }
        Reply::Redirect(status, location) => {
            let headers = format!(
                "HTTP/1.1 {status} Redirect\r\nLocation: {location}\r\nConnection: close\r\n\r\n"
            );
            stream.write_all(headers.as_bytes()).await.unwrap();
        }
        Reply::StalledBody => {
            let headers =
                "HTTP/1.1 200 Fixture\r\nContent-Length: 1000\r\nConnection: close\r\n\r\n";
            stream.write_all(headers.as_bytes()).await.unwrap();
            std::future::pending::<()>().await;
        }
        Reply::ChunkOverflow => {
            let headers =
                "HTTP/1.1 200 Fixture\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
            stream.write_all(headers.as_bytes()).await.unwrap();
            let chunk = "X".repeat(300_000);
            let chunk_line = format!("{:x}\r\n{}\r\n", chunk.len(), chunk);
            stream.write_all(chunk_line.as_bytes()).await.unwrap();
            stream.write_all(b"0\r\n\r\n").await.unwrap();
        }
    }
    stream.shutdown().await.unwrap();
}

async fn send_complete(stream: &mut tokio::io::BufReader<TcpStream>, status: u16, body: &str) {
    let headers = format!(
        "HTTP/1.1 {status} Fixture\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(headers.as_bytes()).await.unwrap();
    stream.write_all(body.as_bytes()).await.unwrap();
}

async fn request(reply: Reply) -> Result<VerdictBatch, ModelError> {
    let stalled = matches!(reply, Reply::StalledHeaders | Reply::StalledBody);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        tokio::time::timeout(Duration::from_secs(5), async move {
            let (stream, _) = listener.accept().await.unwrap();
            serve(stream, reply).await;
        })
        .await
        .expect("HTTP fixture exceeded its deadline");
    });
    let mut options =
        ModelOptions::local(&format!("http://{address}"), "fixture-model").expect("valid endpoint");
    options = options.with_timeout(if stalled {
        Duration::from_millis(100)
    } else {
        Duration::from_secs(2)
    });
    let client = ModelClient::new(options).expect("client builds");
    let packet = census_domain::model::ReviewPacket::new("school:test", "Test");
    let result = tokio::time::timeout(Duration::from_secs(5), client.adjudicate(&packet)).await;
    if stalled {
        server.abort();
        assert!(server.await.unwrap_err().is_cancelled());
    } else {
        server.await.expect("HTTP fixture failed");
    }
    result.expect("model request exceeded the outer deadline")
}

#[tokio::test]
async fn http_503_returns_status_error() {
    match request(Reply::Complete(503, "overloaded"))
        .await
        .unwrap_err()
    {
        ModelError::Status { status } => assert_eq!(status, 503),
        other => panic!("expected HTTP status failure, got {other:?}"),
    }
}

#[tokio::test]
async fn malformed_batch_returns_content_error() {
    let body = r#"{"choices":[{"message":{"content":"PRIVATE_MODEL_SENTINEL_invalid_json"}}]}"#;
    let error = request(Reply::Complete(200, body)).await.unwrap_err();
    assert!(matches!(error, ModelError::Content { .. }));
    assert!(!error.to_string().contains("PRIVATE_MODEL_SENTINEL"));
}

#[tokio::test]
async fn empty_message_returns_empty_error() {
    let body = r#"{"choices":[{"message":{"content":""}}]}"#;
    assert!(matches!(
        request(Reply::Complete(200, body)).await,
        Err(ModelError::Empty)
    ));
}

#[tokio::test]
async fn header_stall_returns_timeout() {
    match request(Reply::StalledHeaders).await.unwrap_err() {
        ModelError::RequestTimeout => {}
        other => panic!("expected timeout, got {other:?}"),
    }
}

#[tokio::test]
async fn body_stall_returns_timeout() {
    match request(Reply::StalledBody).await.unwrap_err() {
        ModelError::RequestTimeout => {}
        other => panic!("expected timeout, got {other:?}"),
    }
}

#[tokio::test]
async fn redirect_is_not_followed() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    assert!(matches!(
        request(Reply::Redirect(302, format!("http://{address}"))).await,
        Err(ModelError::Status { status: 302 })
    ));
    assert!(
        tokio::time::timeout(Duration::from_millis(50), listener.accept())
            .await
            .is_err()
    );
}
#[tokio::test]
async fn chunked_overflow_returns_response_error() {
    let result = request(Reply::ChunkOverflow).await;
    match result.unwrap_err() {
        ModelError::ResponseTooLarge { bytes } => assert!(bytes > 256_000),
        other => panic!("expected response overflow, got {other:?}"),
    }
}
