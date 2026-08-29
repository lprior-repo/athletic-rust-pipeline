use crate::alpha_api::AlphaApiError;
use crate::alpha_api::AlphaApiClientConfig;
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::{AlphaRequest, PaginationConfig};
use crate::alpha_model_raw::RawRankingRecord;
fn make_test_request() -> AlphaRequest {
    AlphaRequest {
        state_id: 12,
        season_id: 2026,
        gender: "m".into(),
        event_short: "100m".into(),
        indoor: false,
        continuation: None,
    }
}

// --- Missing MeetID/MeetName tests ---

#[test]
fn from_flattened_records_errors_on_missing_meet_id() {
    let json = r#"{
        "AthleteID": 1, "AthleteName": "Test", "GradeID": 2,
        "TeamName": "School", "State": "CA",
        "EventShort": "100m", "Measure": "10.5s",
        "ResultDate": "2024-01-01", "SeasonID": 1,
        "MeetName": "Meet"
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
        "AthleteID": 1, "AthleteName": "Test", "GradeID": 2,
        "TeamName": "School", "State": "CA",
        "EventShort": "100m", "Measure": "10.5s",
        "ResultDate": "2024-01-01", "SeasonID": 1, "MeetID": 123
    }"#;
    let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
    let result = rec.to_flattened_records();
    assert!(result.is_err(), "missing MeetName must error");
    let err = result.unwrap_err();
    let msg = err.to_string(); eprintln!("Error: {}", msg); assert!(msg.contains("meet") || msg.contains("required"));
}

#[tokio::test(flavor = "multi_thread")]
async fn single_response_rankings_continuation_is_none() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("POST", "/rankings")
        .match_body(mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"groupedRankings":[],"complete":true,"page":1}"#)
        .create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/rankings".to_owned(),
        nav_info_path: "/nav".to_owned(),
        timeout_seconds: 10,
        max_retries: 0,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/rankings".to_owned()],
        allowed_fields: vec!["AthleteID".into(),"AthleteName".into(),"GradeID".into(),"TeamName".into(),"State".into(),"MeetID".into(),"MeetName".into(),"IDResult".into(),"EventShort".into(),"Measure".into(),"ResultDate".into(),"SeasonID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true,
        permission_reference: "test".into(),
    })
    .expect("client creation must not fail");
    let page = client.rankings(&make_test_request()).await.unwrap();
    assert!(page.continuation.is_none(), "SingleResponse must return None continuation");
}

// --- Bounded capacity and oversized-status mapping tests ---

#[tokio::test(flavor = "multi_thread")]
async fn oversized_5xx_body_maps_to_server_error_exhausted() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("POST", "/api")
        .with_status(503)
        .with_header("content-type", "application/json")
        .with_body(&"x".repeat(9 * 1024 * 1024))
        .create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api".to_owned(),
        nav_info_path: "/nav".to_owned(),
        timeout_seconds: 10,
        max_retries: 0,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".to_owned()],
        allowed_fields: vec!["AthleteID".into(),"AthleteName".into(),"GradeID".into(),"TeamName".into(),"State".into(),"MeetID".into(),"MeetName".into(),"IDResult".into(),"EventShort".into(),"Measure".into(),"ResultDate".into(),"SeasonID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true,
        permission_reference: "test".into(),
    }).expect("client must not fail");
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::ServerErrorExhausted { status: 503, retries: 0 }), "oversized 5xx must return ServerErrorExhausted(status=503), got {:?}", err);
}

#[tokio::test(flavor = "multi_thread")]
async fn oversized_non_2xx_body_maps_to_unexpected_status() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("POST", "/api")
        .with_status(499)
        .with_header("content-type", "application/json")
        .with_body(&"x".repeat(9 * 1024 * 1024))
        .create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api".to_owned(),
        nav_info_path: "/nav".to_owned(),
        timeout_seconds: 10,
        max_retries: 0,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".to_owned()],
        allowed_fields: vec!["AthleteID".into(),"AthleteName".into(),"GradeID".into(),"TeamName".into(),"State".into(),"MeetID".into(),"MeetName".into(),"IDResult".into(),"EventShort".into(),"Measure".into(),"ResultDate".into(),"SeasonID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true,
        permission_reference: "test".into(),
    }).expect("client must not fail");
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    match &err {
        AlphaApiError::UnexpectedStatus { status, body } => {
            assert_eq!(*status, 499, "must preserve non-2xx status");
            assert!(body.contains("too large"), "body must contain 'too large', got: {}", body);
        },
        other => panic!("oversized non-2xx must return UnexpectedStatus, got {:?}", other),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn oversized_2xx_body_still_returns_body_too_large() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("POST", "/api")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(&"x".repeat(9 * 1024 * 1024))
        .create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api".to_owned(),
        nav_info_path: "/nav".to_owned(),
        timeout_seconds: 10,
        max_retries: 0,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".to_owned()],
        allowed_fields: vec!["AthleteID".into(),"AthleteName".into(),"GradeID".into(),"TeamName".into(),"State".into(),"MeetID".into(),"MeetName".into(),"IDResult".into(),"EventShort".into(),"Measure".into(),"ResultDate".into(),"SeasonID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true,
        permission_reference: "test".into(),
    }).expect("client must not fail");
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::BodyTooLarge { limit: 8388608 }), "2xx oversized must return BodyTooLarge, got {:?}", err);
}

// --- Constructor validation regression tests ---

#[test]
fn invalid_config_max_body_bytes_zero() {
    let result = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".into(),
        rankings_path: "/api".into(),
        nav_info_path: "/api".into(),
        timeout_seconds: 30, max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".into()],
        allowed_fields: vec!["AthleteID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 0,
        auth_enabled: true, permission_reference: "test".into(),
    });
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn invalid_config_max_body_bytes_over_8mi() {
    let result = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".into(),
        rankings_path: "/api".into(),
        nav_info_path: "/api".into(),
        timeout_seconds: 30, max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".into()],
        allowed_fields: vec!["AthleteID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8_388_609,
        auth_enabled: true, permission_reference: "test".into(),
    });
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn invalid_config_max_retries_over_5() {
    let result = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".into(),
        rankings_path: "/api".into(),
        nav_info_path: "/api".into(),
        timeout_seconds: 30, max_retries: 6,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".into()],
        allowed_fields: vec!["AthleteID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true, permission_reference: "test".into(),
    });
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn invalid_config_timeout_zero() {
    let result = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".into(),
        rankings_path: "/api".into(),
        nav_info_path: "/api".into(),
        timeout_seconds: 0, max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".into()],
        allowed_fields: vec!["AthleteID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true, permission_reference: "test".into(),
    });
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn invalid_config_timeout_over_300() {
    let result = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".into(),
        rankings_path: "/api".into(),
        nav_info_path: "/api".into(),
        timeout_seconds: 301, max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".into()],
        allowed_fields: vec!["AthleteID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true, permission_reference: "test".into(),
    });
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn invalid_config_retry_delay_below_min() {
    let result = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".into(),
        rankings_path: "/api".into(),
        nav_info_path: "/api".into(),
        timeout_seconds: 30, max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".into()],
        allowed_fields: vec!["AthleteID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 1000, max_retry_delay_ms: 500,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true, permission_reference: "test".into(),
    });
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn invalid_config_retry_delay_over_300k() {
    let result = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".into(),
        rankings_path: "/api".into(),
        nav_info_path: "/api".into(),
        timeout_seconds: 30, max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".into()],
        allowed_fields: vec!["AthleteID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 300_001,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true, permission_reference: "test".into(),
    });
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}
#[test]
fn invalid_config_retry_delay_zero() {
    let result = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".into(),
        rankings_path: "/api".into(),
        nav_info_path: "/api".into(),
        timeout_seconds: 30, max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api".into()],
        allowed_fields: vec!["AthleteID".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 1000, max_retry_delay_ms: 0,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true, permission_reference: "test".into(),
    });
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}
