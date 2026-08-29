use crate::alpha_api::AlphaApiError;
use serde_json::json;
use std::io::{Read, Write};
use crate::alpha_test_helpers::make_test_request;
use crate::alpha_test_helpers::{make_client, make_full_pagination_config, success_body};

#[tokio::test(flavor = "multi_thread")]
async fn http_200_success() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    let mock = server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(success_body())
        .create();
    let client = make_client(&url);
    let page = client.rankings(&make_test_request()).await.unwrap();
    assert!(page.complete);
    assert_eq!(page.records.len(), 1);
    assert_eq!(page.records[0].athlete_id, 1);
    assert_eq!(page.records[0].meet_id, 100);
    assert_eq!(page.records[0].meet_name, "State Finals");
    mock.assert();
}
#[tokio::test(flavor = "multi_thread")]
async fn http_401_immediate_error() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(401)
        .with_body("unauthorised")
        .create();
    let client = make_client(&url);
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::Unauthorized(_)));
}
#[tokio::test(flavor = "multi_thread")]
async fn http_403_immediate_error() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(403)
        .with_body("forbidden")
        .create();
    let client = make_client(&url);
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::Forbidden(_)));
}
#[tokio::test(flavor = "multi_thread")]
async fn http_429_with_retry_after_exhausted() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    for _ in 0..3 {
        server.mock("POST", "/api/v1/tfRankings/GetRankings")
            .with_status(429)
            .with_header("Retry-After", "0")
            .create();
    }
    let client = make_client(&url);
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::RateLimitedExhausted { .. }));
}
#[tokio::test(flavor = "multi_thread")]
async fn http_429_no_retry_after_header() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(429)
        .with_body("")
        .create();
    let client = make_client(&url);
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::RateLimitedNoRetryAfter));
}
#[tokio::test(flavor = "multi_thread")]
async fn http_5xx_bounded_retry_succeeds() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(500).create();
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(500).create();
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(200)
        .with_body(success_body())
        .create();
    let client = make_client(&url);
    let page = client.rankings(&make_test_request()).await.unwrap();
    assert_eq!(page.records.len(), 1);
}
#[tokio::test(flavor = "multi_thread")]
async fn http_5xx_exhausted_retries() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    for _ in 0..3 {
        server.mock("POST", "/api/v1/tfRankings/GetRankings")
            .with_status(503).create();
    }
    let client = make_client(&url);
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::ServerErrorExhausted { status: 503, .. }));
}
#[tokio::test(flavor = "multi_thread")]
async fn http_unexpected_status() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(404)
        .with_body("not found")
        .create();
    let client = make_client(&url);
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::UnexpectedStatus { status: 404, .. }));
}
#[tokio::test(flavor = "multi_thread")]
async fn http_429_retry_after_one_second() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    for _ in 0..3 {
        server.mock("POST", "/api/v1/tfRankings/GetRankings")
            .with_status(429)
            .with_header("Retry-After", "1")
            .create();
    }
    let client = make_full_pagination_config(&url);
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    match err {
        AlphaApiError::RateLimitedExhausted { total_delay_ms, .. } => {
            assert_eq!(total_delay_ms, 2000, "Retry-After: 1s must convert to 1000ms per retry");
        }
        other => panic!("expected RateLimitedExhausted, got {:?}", other),
    }
}

/// Retry-After exceeding 300s operational max returns RateLimitedExhausted immediately.
#[tokio::test(flavor = "multi_thread")]
async fn http_429_retry_after_exceeds_operational_max_returns_exhausted() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{port}/api/v1/tfRankings/GetRankings");
    let handle = tokio::task::spawn_blocking(move || {
        let mut conn = listener.accept().unwrap().0;
        let mut buf = [0u8; 4096];
        conn.read(&mut buf).unwrap();
        let _ = conn.write_all(b"HTTP/1.1 429 Too Many Requests\r\nRetry-After: 999999\r\nContent-Length: 0\r\n\r\n");
    });
    let client = make_client(&url);
    let start = std::time::Instant::now();
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    let elapsed = start.elapsed();
    assert!(matches!(err, AlphaApiError::RateLimitedExhausted { .. }), "must return RateLimitedExhausted, got {:?}", err);
    // Must return immediately (no sleep), not hang for 300+ seconds
    assert!(elapsed.as_secs() < 3, "excessive Retry-After must return immediately, took {elapsed:?}");
    handle.abort();
}

/// Unsupported continuation types (arrays) must return Incomplete, not qParams.
#[tokio::test(flavor = "multi_thread")]
async fn unsupported_continuation_type_returns_incomplete() {
    use crate::alpha_api_client::AlphaApiClient;
    use crate::alpha_model::PaginationConfig;
    use serde_json::json;
    let result = AlphaApiClient::build_qparams(
        &PaginationConfig::NextPage {
            request_page_key: "nextPage".to_owned(),
            has_more_pointer: "/hasMore".to_owned(),
            next_page_pointer: "/nextPage".to_owned(),
        },
        &Some(json!([])),
    );
    assert!(result.is_err(), "array continuation must be rejected");
    let err = result.unwrap_err();
    assert!(matches!(err, AlphaApiError::Incomplete(_)), "must return Incomplete, got {:?}", err);
}

/// Empty string continuation must return Incomplete.
#[tokio::test(flavor = "multi_thread")]
async fn empty_string_continuation_returns_incomplete() {
    use crate::alpha_api_client::AlphaApiClient;
    use crate::alpha_model::PaginationConfig;
    use serde_json::json;
    let result = AlphaApiClient::build_qparams(
        &PaginationConfig::NextPage {
            request_page_key: "nextPage".to_owned(),
            has_more_pointer: "/hasMore".to_owned(),
            next_page_pointer: "/nextPage".to_owned(),
        },
        &Some(json!("")),
    );
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AlphaApiError::Incomplete(_)));
}

/// Null continuation must return Incomplete.
#[tokio::test(flavor = "multi_thread")]
async fn null_continuation_returns_incomplete() {
    use crate::alpha_api_client::AlphaApiClient;
    use crate::alpha_model::PaginationConfig;
    let result = AlphaApiClient::build_qparams(
        &PaginationConfig::NextPage {
            request_page_key: "nextPage".to_owned(),
            has_more_pointer: "/hasMore".to_owned(),
            next_page_pointer: "/nextPage".to_owned(),
        },
        &Some(serde_json::Value::Null),
    );
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AlphaApiError::Incomplete(_)));
}

/// Empty object continuation must return Incomplete.
#[tokio::test(flavor = "multi_thread")]
async fn empty_object_continuation_returns_incomplete() {
    use crate::alpha_api_client::AlphaApiClient;
    use crate::alpha_model::PaginationConfig;
    let result = AlphaApiClient::build_qparams(
        &PaginationConfig::NextPage {
            request_page_key: "nextPage".to_owned(),
            has_more_pointer: "/hasMore".to_owned(),
            next_page_pointer: "/nextPage".to_owned(),
        },
        &Some(json!({})),
    );
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), AlphaApiError::Incomplete(_)));
}

/// SingleResponse mode with continuation token must return Incomplete.
#[tokio::test(flavor = "multi_thread")]
async fn single_response_with_continuation_returns_incomplete() {
    use crate::alpha_api_client::AlphaApiClient;
    use crate::alpha_model::PaginationConfig;
    let result = AlphaApiClient::build_qparams(
        &PaginationConfig::SingleResponse {
            complete_pointer: "/complete".to_owned(),
        },
        &Some(json!("abc123")),
    );
    assert!(result.is_err(), "SingleResponse with continuation must be rejected");
    assert!(matches!(result.unwrap_err(), AlphaApiError::Incomplete(_)));
}
