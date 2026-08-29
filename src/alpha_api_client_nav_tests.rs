use crate::alpha_api::{AlphaApiError, AlphaApiClientConfig};
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
        cap_markers: vec![],
        min_delay_ms: 0,
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
async fn nav_info_rejects_empty_response() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("GET", "/api/v1/tfRankings/GetNavInfo")
       
       .match_query(mockito::Matcher::Any)
 .match_query(mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body("{}")
        .create();

    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api".to_owned(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".to_owned(),
        timeout_seconds: 10,
        max_retries: 0,
        pagination: PaginationConfig::SingleResponse {
            complete_pointer: "/complete".to_owned(),
        },
        allowed_routes: vec!["/api/v1/tfRankings/GetNavInfo".to_owned()],
        allowed_fields: vec!["AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec![],
    }).expect("client must not fail");

    let result = client.nav_info(2024, false).await;
    let err = result.unwrap_err();
    eprintln!("Actual error: {:?}", err);
    assert!(matches!(err, AlphaApiError::Incomplete(_)), "error was: {:?}", err);
}

#[tokio::test(flavor = "multi_thread")]
async fn nav_info_rejects_response_missing_complete_and_page() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("GET", "/api/v1/tfRankings/GetNavInfo")
        
        .match_query(mockito::Matcher::Any)
.with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"someData": "ignored"}"#)
        .match_query(mockito::Matcher::Any)
.create();

    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api".to_owned(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".to_owned(),
        timeout_seconds: 10,
        max_retries: 0,
        pagination: PaginationConfig::SingleResponse {
            complete_pointer: "/complete".to_owned(),
        },
        allowed_routes: vec!["/api/v1/tfRankings/GetNavInfo".to_owned()],
        allowed_fields: vec!["AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec![],
    }).expect("client must not fail");

    let result = client.nav_info(2024, false).await;
    assert!(result.is_err(), "response missing complete/page must be rejected");
}

#[tokio::test(flavor = "multi_thread")]
async fn nav_info_accepts_partial_response_with_complete() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("GET", "/api/v1/tfRankings/GetNavInfo")
        .match_query(mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"state": {"StateID": 1, "State": "CA", "StateName": "California"}, "complete": true, "page": 1}"#)
        .create();

    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api".to_owned(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".to_owned(),
        timeout_seconds: 10,
        max_retries: 0,
        pagination: PaginationConfig::SingleResponse {
            complete_pointer: "/complete".to_owned(),
        },
        allowed_fields: vec![
            "AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into(),
            "MeetID".into(), "MeetName".into(), "IDResult".into(), "EventShort".into(), "Measure".into(),
            "ResultDate".into(), "SeasonID".into(),
        ],
        allowed_routes: vec!["/api/v1/tfRankings/GetNavInfo".to_owned()],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec![],
    }).expect("client must not fail");

    let result = client.nav_info(2024, false).await;
    assert!(result.is_ok(), "partial response with complete must succeed");
}
