use super::*;
use http::{Method, StatusCode};
use restate_sdk::ingress::{Output, RequestTarget, SendStatus};
use restate_sdk::prelude::Json;

#[path = "transport_tests/fixture.rs"]
mod fixture;
#[path = "transport_tests/large.rs"]
mod large;
use fixture::{exchange, request, Reply, WireBody};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const ID: &str = "inv_1aiqX0vFEFNH1Umgre58JiCLgHfTtztYK5";

#[tokio::test]
async fn bounded_transport_preserves_request_route_body_and_duplicate_response_headers(
) -> TestResult {
    let reply = Reply::new("200 OK", WireBody::Bytes(b"binary\0body".to_vec()))
        .header("x-retained", "first")
        .header("x-retained", "second");
    let (captured, response) = exchange(reply, |origin, transport| async move {
        let mut input = request(
            &origin,
            "/restate/call/BrowserSession/key%3A7/fetch?delay=PT1S",
            Method::POST,
        )?;
        input
            .headers_mut()
            .insert("idempotency-key", "original-idempotency".parse()?);
        *input.body_mut() = Bytes::from_static(b"original\0request");
        Ok::<_, Box<dyn std::error::Error>>(CurrentRoute::over(transport).execute(input).await?)
    })
    .await?;
    let response = response?;
    check!(eq; captured.path, "POST /BrowserSession/key%3A7/fetch?delay=PT1S HTTP/1.1\r\n");
    check!(eq; captured.body.as_slice(), b"original\0request");
    check!(captured
        .headers
        .iter()
        .any(|(name, value)| name.eq_ignore_ascii_case("idempotency-key")
            && value == "original-idempotency"));
    check!(eq; response.status(), StatusCode::OK);
    check!(eq; response.version(), http::Version::HTTP_11);
    check!(eq; response.body().as_ref(), b"binary\0body");
    let values: Vec<_> = response
        .headers()
        .get_all("x-retained")
        .iter()
        .map(|value| value.to_str())
        .collect::<Result<_, _>>()?;
    check!(eq; values, vec!["first", "second"]);
    Ok(())
}

#[tokio::test]
async fn declared_control_overflow_refuses_before_waiting_for_the_body() -> TestResult {
    let reply = Reply::new("202 Accepted", WireBody::Hold)
        .header("content-length", "65537")
        .header("x-restate-id", ID);
    let (_, outcome) = exchange(reply, |origin, transport| async move {
        let input = request(
            &origin,
            "/restate/send/NationalCensus/original/run",
            Method::POST,
        )?;
        Ok::<_, Box<dyn std::error::Error>>(CurrentRoute::over(transport).execute(input).await)
    })
    .await?;
    let error = outcome?.err().ok_or("oversized acknowledgement accepted")?;
    let IngressError::Refused(refusal) = error else {
        return Err("wrong overflow type".into());
    };
    check!(eq; refusal.kind, AdmissionKind::DeclaredBodyBytes);
    check!(eq; refusal.invocation_id.as_deref(), Some(ID));
    check!(eq; refusal.status, 202);
    Ok(())
}

#[tokio::test]
async fn chunked_and_missing_length_control_overflow_are_not_truncated() -> TestResult {
    for body in [
        WireBody::Chunked(vec![vec![b'x'; 65_537]]),
        WireBody::Undeclared(vec![b'x'; 65_537]),
    ] {
        let reply = Reply::new("200 OK", body).refusing_body();
        let (_, outcome) = exchange(reply, |origin, transport| async move {
            let input = request(
                &origin,
                "/restate/call/Census/publication_capabilities",
                Method::POST,
            )?;
            Ok::<_, Box<dyn std::error::Error>>(CurrentRoute::over(transport).execute(input).await)
        })
        .await?;
        let error = outcome?
            .err()
            .ok_or("oversized streamed response accepted")?;
        check!(matches!(
            error,
            IngressError::Refused(ResponseRefusal {
                kind: AdmissionKind::StreamedBodyBytes,
                limit: 65_536,
                ..
            })
        ));
    }
    Ok(())
}

#[tokio::test]
async fn truncated_http_failure_preserves_known_status_invocation_and_transport_source(
) -> TestResult {
    let reply = Reply::new(
        "500 Internal Server Error",
        WireBody::Undeclared(b"short".to_vec()),
    )
    .header("content-length", "100")
    .header("x-restate-id", "inv_foreign123");
    let (_, outcome) = exchange(reply, |origin, transport| async move {
        let input = request(&origin, &format!("/restate/output/{ID}"), Method::GET)?;
        Ok::<_, Box<dyn std::error::Error>>(CurrentRoute::over(transport).execute(input).await)
    })
    .await?;
    let error = outcome?
        .err()
        .ok_or("truncated failure returned a response")?;
    let IngressError::ResponseTransport {
        status,
        invocation_id,
        source,
    } = &error
    else {
        return Err("truncated response lost transport provenance".into());
    };
    check!(eq; *status, Some(500));
    check!(eq; invocation_id.as_deref(), Some(ID));
    check!(source.is_decode());
    check!(std::error::Error::source(&error)
        .is_some_and(|cause| cause.downcast_ref::<reqwest::Error>().is_some()));
    check!(!is_admission_error(
        &restate_sdk::ingress::ClientError::Transport {
            source: Box::new(error),
        }
    ));
    Ok(())
}

#[tokio::test]
async fn invalid_http_headers_preserve_requested_invocation_without_inventing_status() -> TestResult
{
    let reply = Reply::new("200 OK\r\ninvalid header line", WireBody::Hold);
    let (_, outcome) = exchange(reply, |origin, transport| async move {
        let input = request(&origin, &format!("/restate/output/{ID}"), Method::GET)?;
        Ok::<_, Box<dyn std::error::Error>>(CurrentRoute::over(transport).execute(input).await)
    })
    .await?;
    let error = outcome?
        .err()
        .ok_or("invalid response headers were accepted")?;
    let IngressError::ResponseTransport {
        status,
        invocation_id,
        source,
    } = &error
    else {
        return Err("invalid response lost transport provenance".into());
    };
    check!(eq; *status, None);
    check!(eq; invocation_id.as_deref(), Some(ID));
    check!(source.is_request());
    check!(!is_admission_error(
        &restate_sdk::ingress::ClientError::Transport {
            source: Box::new(error),
        }
    ));
    Ok(())
}

#[tokio::test]
async fn sdk_send_and_output_semantics_survive_the_bounded_executor() -> TestResult {
    let acknowledgement = serde_json::json!({"invocationId": ID, "status": "Accepted"}).to_string();
    let reply = Reply::new(
        "202 Accepted",
        WireBody::Bytes(acknowledgement.into_bytes()),
    )
    .header("x-restate-id", ID);
    let (captured, outcome) = exchange(reply, |origin, transport| async move {
        let client = client(origin.parse()?, transport)?;
        let sent = client
            .request::<Json<serde_json::Value>, Json<serde_json::Value>>(
                RequestTarget::object("NationalCensus", "same-key", "run"),
                Json(serde_json::json!({"original": true})),
            )
            .send()
            .await?;
        check!(matches!(sent.send_status(), SendStatus::Accepted));
        check!(eq; sent.invocation_handle().invocation_id(), ID);
        Ok::<_, Box<dyn std::error::Error>>(())
    })
    .await?;
    outcome?;
    check!(eq; captured.path, "POST /NationalCensus/same-key/run/send HTTP/1.1\r\n");
    for (status, body, ready) in [
        ("470 Not Ready", "{\"message\":\"pending\"}", false),
        ("200 OK", "{\"retained\":true}", true),
    ] {
        let reply = Reply::new(status, WireBody::Bytes(body.as_bytes().to_vec()));
        let (captured, outcome) = exchange(reply, |origin, transport| async move {
            let client = client(origin.parse()?, transport)?;
            let response = client
                .invocation_handle::<Json<serde_json::Value>>(ID)
                .output()
                .await?;
            check!(eq; matches!(response.body(), Output::Ready(_)), ready);
            Ok::<_, Box<dyn std::error::Error>>(())
        })
        .await?;
        outcome?;
        check!(eq; captured.path, format!("GET /restate/invocation/{ID}/output HTTP/1.1\r\n"));
    }
    Ok(())
}

#[tokio::test]
async fn small_error_status_and_body_reach_the_sdk_unchanged() -> TestResult {
    let body = b"{\"message\":\"original refusal\"}";
    let reply =
        Reply::new("404 Not Found", WireBody::Bytes(body.to_vec())).header("x-original", "kept");
    let (_, outcome) = exchange(reply, |origin, transport| async move {
        let client = client(origin.parse()?, transport)?;
        let error = client.invocation_handle::<Json<serde_json::Value>>(ID).output().await.err().ok_or("missing output accepted")?;
        let response = error.response().ok_or("SDK lost error response")?;
        check!(eq; response.status(), StatusCode::NOT_FOUND);
        check!(eq; response.body().as_ref(), body);
        check!(eq; response.headers().get("x-original").and_then(|value| value.to_str().ok()), Some("kept"));
        Ok::<_, Box<dyn std::error::Error>>(())
    }).await?;
    outcome?;
    Ok(())
}

#[tokio::test]
async fn output_refusal_preserves_requested_id_without_trusting_response_identity() -> TestResult {
    for header in [None, Some("inv_foreign123")] {
        let reply = Reply::new("470 Not Ready", WireBody::Hold).header("content-length", "262145");
        let reply = match header {
            Some(id) => reply.header("x-restate-id", id),
            None => reply,
        };
        let (_, outcome) = exchange(reply, |origin, transport| async move {
            let input = request(&origin, &format!("/restate/output/{ID}"), Method::GET)?;
            Ok::<_, Box<dyn std::error::Error>>(CurrentRoute::over(transport).execute(input).await)
        })
        .await?;
        let error = outcome?.err().ok_or("oversized output accepted")?;
        let IngressError::Refused(refusal) = error else {
            return Err("wrong output refusal type".into());
        };
        check!(eq; refusal.invocation_id.as_deref(), Some(ID));
        check!(eq; refusal.status, 470);
        check!(eq; refusal.kind, AdmissionKind::DeclaredBodyBytes);
    }
    Ok(())
}

#[tokio::test]
async fn response_without_body_chunks_preserves_the_configured_request_deadline() -> TestResult {
    let reply = Reply::new("200 OK", WireBody::Hold).header("content-length", "1");
    let (captured, outcome) = exchange(reply, |origin, _| async move {
        let transport = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_millis(100))
            .build()?;
        let input = request(&origin, &format!("/restate/output/{ID}"), Method::GET)?;
        Ok::<_, Box<dyn std::error::Error>>(CurrentRoute::over(transport).execute(input).await)
    })
    .await?;
    check!(eq; captured.path, format!("GET /restate/invocation/{ID}/output HTTP/1.1\r\n"));
    let error = outcome?.err().ok_or("held response did not time out")?;
    let IngressError::ResponseTransport {
        status,
        invocation_id,
        source,
    } = &error
    else {
        return Err("response timeout lost transport provenance".into());
    };
    check!(eq; *status, Some(200));
    check!(eq; invocation_id.as_deref(), Some(ID));
    check!(source.is_timeout());
    check!(!is_admission_error(
        &restate_sdk::ingress::ClientError::Transport {
            source: Box::new(error),
        }
    ));
    Ok(())
}

#[tokio::test]
async fn http_parser_header_overflow_is_typed_terminal_and_keeps_the_requested_invocation(
) -> TestResult {
    let reply = (0..101).fold(
        Reply::new("200 OK", WireBody::Hold).header("x-restate-id", "inv_foreign123"),
        |reply, _| reply.header("x-extra", "v"),
    );
    let (captured, outcome) = exchange(reply, |origin, transport| async move {
        let client = client(origin.parse()?, transport)?;
        Ok::<_, Box<dyn std::error::Error>>(
            client
                .invocation_handle::<Json<serde_json::Value>>(ID)
                .output()
                .await,
        )
    })
    .await?;
    check!(eq; captured.path, format!("GET /restate/invocation/{ID}/output HTTP/1.1\r\n"));
    let error = outcome?.err().ok_or("parser header excess admitted")?;
    check!(is_admission_error(&error));
    let restate_sdk::ingress::ClientError::Transport { source } = error else {
        return Err("wrong SDK parser refusal type".into());
    };
    let Some(IngressError::ParserSize {
        status,
        observed,
        invocation_id,
        ..
    }) = source.downcast_ref::<IngressError>()
    else {
        return Err("parser size refusal lost its typed cause".into());
    };
    check!(status.is_none());
    check!(observed.is_none());
    check!(eq; invocation_id.as_deref(), Some(ID));
    Ok(())
}
