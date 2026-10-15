use super::lookup_workflow;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const ID: &str = "inv_1aiqX0vFEFNH1Umgre58JiCLgHfTtztYK5";
const KEY: &str = "national:2026-27:retained-scope:1:resume:7";

async fn exchange(
    response: String,
) -> TestResult<(Vec<(String, Vec<u8>)>, anyhow::Result<String>)> {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let origin = format!("http://{}/", listener.local_addr()?);
    let (finished, mut completion) = tokio::sync::oneshot::channel::<()>();
    let (captured, outcome) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(
            async {
                let mut captured = Vec::new();
                loop {
                    let socket = tokio::select! {
                        biased;
                        accepted = listener.accept() => accepted?.0,
                        _ = &mut completion => return Ok::<_, Box<dyn std::error::Error>>(captured),
                    };
                    if captured.len() >= 2 {
                        return Err("lookup fixture exceeded request bound".into());
                    }
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
                    if length > 16384 {
                        return Err("lookup fixture request too large".into());
                    }
                    let mut body = vec![0; length];
                    socket.read_exact(&mut body).await?;
                    captured.push((path, body));
                    socket.get_mut().write_all(response.as_bytes()).await?;
                    socket.get_mut().shutdown().await?;
                }
            },
            async {
                let result = lookup_workflow(&origin, "NationalCensus", KEY).await;
                drop(finished);
                result
            }
        )
    })
    .await?;
    Ok((captured?, outcome))
}

fn response(status: &str, body: &str) -> String {
    format!("HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nLocation: /unexpected-redirect\r\nConnection: close\r\n\r\n{body}", body.len())
}

fn assert_target(captured: &[(String, Vec<u8>)]) -> TestResult {
    check!(eq; captured.len(), 1);
    let (path, body) = captured.first().ok_or("lookup request absent")?;
    check!(eq; path, "POST /restate/lookup HTTP/1.1\r\n");
    check!(eq; serde_json::from_slice::<serde_json::Value>(body)?, serde_json::json!({
        "target": "workflow", "workflowName": "NationalCensus", "workflowKey": KEY
    }));
    Ok(())
}

#[tokio::test]
async fn workflow_lookup_posts_exact_target_and_accepts_headerless_server_response() -> TestResult {
    let body = serde_json::json!({"invocationId": ID}).to_string();
    let (captured, outcome) = exchange(response("200 OK", &body)).await?;
    assert_target(&captured)?;
    check!(eq; outcome?, ID);
    Ok(())
}

#[tokio::test]
async fn workflow_lookup_refuses_status_redirect_and_malformed_or_oversized_ids() -> TestResult {
    let valid = serde_json::json!({"invocationId": ID}).to_string();
    for (status, body) in [
        ("404 Not Found", valid.clone()),
        ("302 Found", valid.clone()),
        ("201 Created", valid),
        ("200 OK", "not json".into()),
        ("200 OK", "{}".into()),
        ("200 OK", "{\"invocationId\":null}".into()),
        ("200 OK", "{\"invocationId\":\"inv_\"}".into()),
        ("200 OK", "{\"invocationId\":\"inv_abc/foreign\"}".into()),
        ("200 OK", "{\"invocationId\":\"national:key\"}".into()),
        (
            "200 OK",
            "{\"invocationId\":\"inv_abc\",\"invocationId\":\"inv_def\"}".into(),
        ),
        (
            "200 OK",
            serde_json::json!({"invocationId": format!("inv_{}", "a".repeat(125))}).to_string(),
        ),
    ] {
        let (captured, outcome) = exchange(response(status, &body)).await?;
        assert_target(&captured)?;
        check!(outcome.is_err(), "{status} {body}");
    }
    Ok(())
}

#[tokio::test]
async fn workflow_lookup_accepts_exact_byte_bound_and_refuses_declared_or_streamed_overflow(
) -> TestResult {
    let body = serde_json::json!({"invocationId": ID}).to_string();
    let exact = format!("{}{}", " ".repeat(4096 - body.len()), body);
    let (captured, outcome) = exchange(response("200 OK", &exact)).await?;
    assert_target(&captured)?;
    check!(eq; outcome?, ID);
    let overflow = format!(" {exact}");
    for wire in [response("200 OK", &overflow), format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n{:x}\r\n{overflow}\r\n0\r\n\r\n", overflow.len()
    )] {
        let (captured, outcome) = exchange(wire).await?;
        assert_target(&captured)?;
        let error = outcome.err().ok_or("oversized lookup accepted")?;
        check!(format!("{error:#}").contains("exceeds its byte bound"), "{error:#}");
    }
    Ok(())
}

#[tokio::test]
async fn workflow_lookup_validates_origin_and_target_before_connecting() -> TestResult {
    for (origin, workflow, key) in [
        ("http://192.0.2.1/", "NationalCensus", KEY),
        ("http://localhost/private/", "NationalCensus", KEY),
        ("http://localhost:1/", "", KEY),
        ("http://localhost:1/", "NationalCensus", ""),
    ] {
        let error = lookup_workflow(origin, workflow, key)
            .await
            .err()
            .ok_or("invalid lookup accepted")?;
        check!(
            !format!("{error:#}").contains("workflow invocation lookup failed"),
            "{error:#}"
        );
    }
    Ok(())
}
