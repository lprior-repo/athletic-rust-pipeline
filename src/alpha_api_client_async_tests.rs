use std::time::Duration;

use crate::alpha_api::AlphaApiError;
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_api::AlphaApiClientConfig;
use crate::alpha_model::PaginationConfig;
use crate::alpha_test_helpers::make_test_request;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::alpha_test_helpers::{make_client, make_client_with_fields, make_full_pagination_config, success_body};

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

#[tokio::test(flavor = "multi_thread")]
async fn rankings_continuation_next_page_mode() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    let mock = server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(200).with_header("content-type", "application/json")
        .with_body(r#"{"groupedRankings":[[{"AthleteID":1,"AthleteName":"A","GradeID":1,"TeamName":"S","State":"CA","EventShort":"100m","IDResult":105,"Measure":"Seconds","ResultDate":"2024-01-01T12:00:00Z","SeasonID":2024,"Wind":"0.5","MeetID":123,"MeetName":"Meet A"}]],"page":2,"complete":false,"continuation":{"page":2,"complete":false},"hasMore":true,"nextPage":"page=3"}"#)
        .create();

    let client = make_full_pagination_config(&url);
    let req = make_test_request();
    let page = client.rankings(&req).await.expect("rankings must succeed");
    assert_eq!(page.records.len(), 1, "must have 1 record");
    assert!(!page.complete, "hasMore=true => incomplete");
    assert!(page.continuation.is_some(), "continuation must be extracted from nextPage pointer");
    let cont = page.continuation.unwrap();
    assert_eq!(cont.as_str().unwrap(), "page=3", "continuation must be next page value");
    mock.assert();
}

#[tokio::test(flavor = "multi_thread")]
async fn rankings_continuation_single_response_mode() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    let mock = server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(200).with_header("content-type", "application/json")
        .with_body(r#"{"groupedRankings":[[{"AthleteID":1,"AthleteName":"A","GradeID":1,"TeamName":"S","State":"CA","EventShort":"100m","IDResult":105,"Measure":"Seconds","ResultDate":"2024-01-01T12:00:00Z","SeasonID":2024,"Wind":"0.5","MeetID":123,"MeetName":"Meet A"}]],"page":1,"complete":true,"hasMore":false}"#)
        .create();

    let client = make_client(&url);
    let req = make_test_request();
    let page = client.rankings(&req).await.expect("rankings must succeed");
    assert!(page.complete, "complete=true => done");
    mock.assert();
}

#[tokio::test(flavor = "multi_thread")]
async fn rankings_truncated_has_more_no_next_pointer() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"groupedRankings":[[{"AthleteID":1,"AthleteName":"A","GradeID":1,"TeamName":"S","State":"CA","EventShort":"100m","IDResult":105,"Measure":"Seconds","ResultDate":"2024-01-01T12:00:00Z","SeasonID":2024,"Wind":"0.5","MeetID":123,"MeetName":"Meet A"}]],"hasMore":true}"#)
        .create();

    let client = make_full_pagination_config(&url);
    let req = make_test_request();
    let result = client.rankings(&req).await;
    assert!(result.is_err(), "hasMore=true without nextPage must error");
}

#[tokio::test(flavor = "multi_thread")]
async fn enforce_response_allowed_fields_missing_required() {
    let client = make_client_with_fields("http://127.0.0.1", vec!["AthleteID", "AthleteName"]);
    let value = serde_json::json!({"groupedRankings":[]});
    let result = client.enforce_response_allowed_fields(value);
    assert!(result.is_err(), "should error when required fields are missing");
}

#[tokio::test(flavor = "multi_thread")]
async fn enforce_response_allowed_fields_all_required_present() {
    let client = make_client_with_fields(
        "http://127.0.0.1",
        vec!["AthleteID", "AthleteName", "GradeID", "TeamName", "State", "MeetID", "MeetName", "IDResult", "EventShort", "Measure", "ResultDate", "SeasonID"],
    );
    let value = serde_json::json!({"groupedRankings":[]});
    let result = client.enforce_response_allowed_fields(value);
    assert!(result.is_ok(), "should succeed when all required fields are present");
}

#[tokio::test(flavor = "multi_thread")]
async fn enforce_response_allowed_fields_empty_fails_closed() {
    let client = make_client_with_fields("http://127.0.0.1", vec![]);
    let value = serde_json::json!({"groupedRankings":[]});
    let result = client.enforce_response_allowed_fields(value);
    assert!(result.is_err(), "empty allowed_fields must fail closed");
}

#[tokio::test(flavor = "multi_thread")]
async fn body_timeout_cancellation_no_orphan() {
    // Raw TCP server sends 200 headers, delays body by 6s.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        loop {
            let mut s = listener.accept().await.unwrap().0;
            tokio::task::spawn(async move {
                let _ = s.read(&mut [0u8; 8192]).await;
                let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n").await;
                tokio::time::sleep(Duration::from_secs(6)).await;
                let _ = s.write_all(b"{}").await;
            });
        }
    });
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: format!("http://{addr}/api/v1/tfRankings/GetRankings"),
        rankings_path: "/api/v1/tfRankings/GetRankings".into(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".into(),
        timeout_seconds: 2, max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api/v1/tfRankings/GetRankings".into()],
        allowed_fields: vec!["AthleteID","AthleteName","GradeID","TeamName","State","MeetID","MeetName","IDResult","EventShort","Measure","ResultDate","SeasonID"].into_iter().map(String::from).collect(),
        max_concurrent_requests: 1, min_delay_ms: 0, cap_markers: vec![],
    }).expect("client creation must not fail");
    let start = std::time::Instant::now();
    let result = client.rankings(&make_test_request()).await;
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_secs(5), "retried bounded attempts, elapsed {:?}", elapsed);
    assert!(elapsed < Duration::from_secs(10), "did not hang, elapsed {:?}", elapsed);
    match result { Err(AlphaApiError::Timeout { .. }) => {}, other => panic!("expected Timeout, got {:?}", other), }
}
