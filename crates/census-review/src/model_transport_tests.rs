use std::time::Duration;

use census_domain::model::{ReviewPacket, ReviewVerdictKind, VerdictBatch};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use super::{ModelClient, ModelError, ModelOptions};

enum Reply {
    Complete(u16, &'static str),
    Truncated,
    StalledHeaders,
    StalledBody,
}

async fn serve(mut stream: TcpStream, reply: Reply) {
    let mut reader = BufReader::new(&mut stream);
    let mut length = 0;
    loop {
        let mut line = String::new();
        assert_ne!(reader.read_line(&mut line).await.unwrap(), 0);
        if line == "\r\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse::<usize>().unwrap();
            }
        }
    }
    let mut body = vec![0; length];
    reader.read_exact(&mut body).await.unwrap();
    drop(reader);
    if matches!(reply, Reply::StalledHeaders) {
        std::future::pending::<()>().await;
    }
    let (status, body, length) = match reply {
        Reply::Complete(status, body) => (status, body, body.len()),
        _ => (200, "partial", 1000),
    };
    let headers = format!(
        "HTTP/1.1 {status} Fixture\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(headers.as_bytes()).await.unwrap();
    stream.write_all(body.as_bytes()).await.unwrap();
    if matches!(reply, Reply::StalledBody) {
        std::future::pending::<()>().await;
    }
    stream.shutdown().await.unwrap();
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
    let mut options = ModelOptions::local(&format!("http://{address}"), "fixture-model");
    options.timeout = if stalled {
        Duration::from_millis(100)
    } else {
        Duration::from_secs(2)
    };
    let client = ModelClient {
        http: reqwest::Client::builder()
            .no_proxy()
            .timeout(options.timeout)
            .build()
            .unwrap(),
        options,
    };
    let packet = ReviewPacket::new("school:madison-west", "Madison West");
    let result = tokio::time::timeout(Duration::from_secs(3), client.adjudicate(&packet)).await;
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
        ModelError::Status { status, body, .. } => {
            assert_eq!(status, 503);
            assert_eq!(body, "overloaded");
        }
        other => panic!("expected HTTP status failure, got {other:?}"),
    }
}

#[tokio::test]
async fn malformed_batch_returns_content_error() {
    let body = r#"{"choices":[{"message":{"content":"not a verdict batch"}}]}"#;
    match request(Reply::Complete(200, body)).await.unwrap_err() {
        ModelError::Content { content, .. } => assert_eq!(content, "not a verdict batch"),
        other => panic!("expected malformed verdict failure, got {other:?}"),
    }
}

#[tokio::test]
async fn empty_message_returns_empty_error() {
    let body = r#"{"choices":[{"message":{"content":""}}]}"#;
    assert!(matches!(
        request(Reply::Complete(200, body)).await,
        Err(ModelError::Empty { status: 200, .. })
    ));
}

#[tokio::test]
async fn header_stall_returns_timeout_request_error() {
    match request(Reply::StalledHeaders).await.unwrap_err() {
        ModelError::Request { source, .. } => assert!(source.is_timeout()),
        other => panic!("expected header timeout, got {other:?}"),
    }
}

#[tokio::test]
async fn body_stall_returns_timeout_request_error() {
    match request(Reply::StalledBody).await.unwrap_err() {
        ModelError::Request { source, .. } => assert!(source.is_timeout()),
        other => panic!("expected body timeout, got {other:?}"),
    }
}

#[tokio::test]
async fn truncated_body_returns_request_error() {
    match request(Reply::Truncated).await.unwrap_err() {
        ModelError::Request { source, .. } => assert!(!source.is_timeout()),
        other => panic!("expected truncated-body transport failure, got {other:?}"),
    }
}

#[tokio::test]
async fn valid_verdict_returns_batch() {
    let body = r#"{"choices":[{"message":{"content":"{\"subject_id\":\"school:madison-west\",\"verdicts\":[{\"case_id\":\"case-1\",\"kind\":\"value_proposed\",\"field\":\"state\",\"value\":\"WI\",\"confidence\":90,\"rationale\":\"source identifies Wisconsin\"}]}"}}]}"#;
    let batch = request(Reply::Complete(200, body)).await.unwrap();
    assert_eq!(batch.subject_id, "school:madison-west");
    assert_eq!(batch.verdicts.len(), 1);
    let verdict = &batch.verdicts[0];
    assert_eq!(verdict.case_id, "case-1");
    assert_eq!(verdict.kind, ReviewVerdictKind::ValueProposed);
    assert_eq!(verdict.field.as_deref(), Some("state"));
    assert_eq!(verdict.value.as_deref(), Some("WI"));
}
