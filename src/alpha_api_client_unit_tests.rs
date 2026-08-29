use crate::alpha_api::{AlphaApiError, AlphaApiClientConfig};
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::{AlphaRequest, PaginationConfig};
use crate::alpha_model_raw::RawRankingRecord;
fn make_client(server_url: &str) -> AlphaApiClient {
    let config = AlphaApiClientConfig {
        base_url: server_url.to_owned(),
        rankings_path: "/api/v1/tfRankings/GetRankings".to_owned(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".to_owned(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse {
            complete_pointer: "/complete".to_owned(),
        },
        allowed_routes: vec![
            "/api/v1/tfRankings/GetRankings".to_owned(),
            "/api/v1/tfRankings/GetNavInfo".to_owned(),
        ],
        allowed_fields: vec!["AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec![],
    };
    AlphaApiClient::new(config).expect("client creation must not fail")
}
fn make_test_request() -> AlphaRequest {
    AlphaRequest {
        state_id: 12,
        season_id: 2026,
        gender: "m".to_owned(),
        event_short: "100m".to_owned(),
        indoor: false,
        continuation: None,
    }
}
fn success_body() -> &'static str {
    r#"{
        "groupedRankings": [[{
            "AthleteID": 1,
            "AthleteName": "Test",
            "GradeID": 2,
            "TeamName": "School",
            "State": "CA",
            "Results": [{
                "MeetID": 100,
                "MeetName": "State Finals",
                "IDResult": 500,
                "EventShort": "100m",
                "Measure": "10.55",
                "ResultDate": "2026-06-15",
                "SeasonID": 2026,
                "Wind": null
            }]
        }]],
        "page": 1,
        "complete": true,
        "continuation": null
    }"#
}
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
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(429)
        .with_header("Retry-After", "1")
        .create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api/v1/tfRankings/GetRankings".to_owned(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".to_owned(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".to_owned() },
        allowed_routes: vec!["/api/v1/tfRankings/GetRankings".into()],
        allowed_fields: vec!["AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec![],
    }).expect("client creation must not fail");
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    match err {
        AlphaApiError::RateLimitedExhausted { total_delay_ms, .. } => {
            assert_eq!(total_delay_ms, 2000, "Retry-After: 1s must convert to 1000ms per retry");
        }
        other => panic!("expected RateLimitedExhausted, got {:?}", other),
    }
}
// --- Constructor error tests ---

#[test]
fn new_returns_error_on_invalid_config() {
    let config = AlphaApiClientConfig {
        base_url: "http://localhost:9999".to_owned(),
        rankings_path: "/api".to_owned(),
        nav_info_path: "/nav".to_owned(),
        timeout_seconds: 1,
        max_retries: 0,
        pagination: PaginationConfig::SingleResponse {
            complete_pointer: "/complete".to_owned(),
        },
        allowed_routes: vec!["/api".to_owned()],
        allowed_fields: vec!["AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec![],
    };
    // This must not panic — it returns Result.
    let result = AlphaApiClient::new(config);
    assert!(result.is_ok(), "new() must return Ok with valid config");
}

// --- Missing MeetID/MeetName tests ---

#[test]
fn from_flattened_records_errors_on_missing_meet_id() {
    let json = r#"{
        "AthleteID": 1,
        "AthleteName": "Test",
        "GradeID": 2,
        "TeamName": "School",
        "State": "CA",
        "EventShort": "100m",
        "MeetName": "Meet A"
    }"#;
    let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
    let result = rec.to_flattened_records();
    assert!(result.is_err(), "missing MeetID must error");
    let err = result.unwrap_err();
    let msg = err.to_string(); eprintln!("Error: {}", msg); assert!(msg.contains("meet") || msg.contains("required"));
}

#[test]
fn from_flattened_records_errors_on_missing_meet_name() {
    let json = r#"{
        "AthleteID": 1,
        "AthleteName": "Test",
        "GradeID": 2,
        "TeamName": "School",
        "State": "CA",
        "EventShort": "100m",
        "MeetID": 123
    }"#;
    let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
    let result = rec.to_flattened_records();
    assert!(result.is_err(), "missing MeetName must error");
    let err = result.unwrap_err();
    let msg = err.to_string(); eprintln!("Error: {}", msg); assert!(msg.contains("meet") || msg.contains("required"));
}
