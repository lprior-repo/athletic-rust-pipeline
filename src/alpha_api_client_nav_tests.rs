use crate::alpha_api::{AlphaApiError, AlphaApiClientConfig};
use std::io::{Read, Write};
use crate::alpha_test_helpers::{make_client, make_test_request, success_body};
use crate::alpha_model::PaginationConfig;
use crate::alpha_api_client::AlphaApiClient;
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
async fn http_401_immediate_error_raw_tcp() {
    // Raw TCP server sends 401 header, delays body to verify
    // the client returns Unauthorized before body is read (no retry).
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{port}/api/v1/tfRankings/GetRankings");
    let retry_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let retry_count_clone = retry_count.clone();
    let _handle = tokio::task::spawn_blocking(move || {
        let mut conn = listener.accept().unwrap().0;
        let mut buf = [0u8; 4096];
        let _ = conn.read(&mut buf);
        retry_count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // Send 401 header, then delay body to prove Unauthorized returned first.
        let _ = conn.write_all(b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 9999\r\n\r\n");
        std::thread::sleep(std::time::Duration::from_secs(5));
    });
    let client = make_client(&url);
    let start = std::time::Instant::now();
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    let elapsed = start.elapsed();
    assert!(matches!(err, AlphaApiError::Unauthorized(_)));
    assert!(elapsed.as_secs() < 2, "Unauthorized must return before body delay, got {elapsed:?}");
    assert_eq!(retry_count.load(std::sync::atomic::Ordering::SeqCst), 1, "must not retry on 401");
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
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api/v1/tfRankings/GetRankings".to_owned(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".to_owned(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".to_owned() },
        allowed_fields: vec![
            "AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into(),
            "MeetID".into(), "MeetName".into(), "IDResult".into(), "EventShort".into(), "Measure".into(),
            "ResultDate".into(), "SeasonID".into(),
        ],
        allowed_routes: vec!["/api/v1/tfRankings/GetRankings".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true,
        permission_reference: "test".into(),
    }).expect("client creation must not fail");
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    match err {
        AlphaApiError::RateLimitedExhausted { total_delay_ms, .. } => {
            assert_eq!(total_delay_ms, 2000, "Retry-After: 1s must convert to 1000ms per retry");
        }
        other => panic!("expected RateLimitedExhausted, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn http_body_oversize_before_buffer_rejected() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let url = format!("http://127.0.0.1:{port}/api/v1/tfRankings/GetRankings");
    let _handle = tokio::task::spawn_blocking(move || {
        let mut conn = listener.accept().unwrap().0;
        let mut buf = [0u8; 4096];
        let _ = conn.read(&mut buf);
        // Send 200 OK with body larger than 8MB limit
        let large_body = "x".repeat(9 * 1024 * 1024);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\n\r\n{}",
            large_body.len(),
            large_body
        );
        let _ = conn.write_all(response.as_bytes());
    });
    let client = make_client(&url);
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::BodyTooLarge { .. }), "oversize body must return BodyTooLarge, got {:?}", err);
}

#[tokio::test(flavor = "multi_thread")]
async fn http_429_retry_after_over_bound_no_sleep() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    // Retry-After: 500 exceeds MAX_RETRY_AFTER_SECONDS (300)
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(429)
        .with_header("Retry-After", "500")
        .create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api/v1/tfRankings/GetRankings".to_owned(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".to_owned(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".to_owned() },
        allowed_fields: vec![
            "AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into(),
            "MeetID".into(), "MeetName".into(), "IDResult".into(), "EventShort".into(), "Measure".into(),
            "ResultDate".into(), "SeasonID".into(),
        ],
        allowed_routes: vec!["/api/v1/tfRankings/GetRankings".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true,
        permission_reference: "test".into(),
    }).expect("client must not fail");
    let start = std::time::Instant::now();
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    let elapsed = start.elapsed();
    // Should return immediately (no sleep) since Retry-After exceeds 300s
    assert!(elapsed.as_millis() < 500, "should not sleep for Retry-After over bound, got {:?}", elapsed);
    match err {
        AlphaApiError::RateLimitedExhausted { .. } => {}
        other => panic!("expected RateLimitedExhausted, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn http_429_retry_after_over_config_bound_no_sleep() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    // Retry-After: 50 (50s) exceeds max_retry_delay_ms=10000/1000=10s
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(429)
        .with_header("Retry-After", "50")
        .create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api/v1/tfRankings/GetRankings".to_owned(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".to_owned(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".to_owned() },
        allowed_fields: vec![
            "AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into(),
            "MeetID".into(), "MeetName".into(), "IDResult".into(), "EventShort".into(), "Measure".into(),
            "ResultDate".into(), "SeasonID".into(),
        ],
        allowed_routes: vec!["/api/v1/tfRankings/GetRankings".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 10_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true,
        permission_reference: "test".into(),
    }).expect("client must not fail");
    let start = std::time::Instant::now();
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    let elapsed = start.elapsed();
    // Should return immediately (no sleep) since Retry-After exceeds config bound
    assert!(elapsed.as_millis() < 500, "should not sleep for Retry-After over config bound, got {:?}", elapsed);
    match err {
        AlphaApiError::RateLimitedExhausted { .. } => {}
        other => panic!("expected RateLimitedExhausted, got {:?}", other),
    }
}
