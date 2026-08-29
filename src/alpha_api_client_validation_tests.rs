use crate::alpha_api::AlphaApiError;
use crate::alpha_api::AlphaApiClientConfig;
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::{AlphaRequest, PaginationConfig};
use crate::alpha_model_raw::{RawRankingRecord, RawRankingsResponse};
use crate::alpha_test_helpers::make_full_pagination_config;
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


// --- Continuation numeric value validation tests ---

#[test]
fn build_qparams_rejects_zero_continuation() {
    let pagination = PaginationConfig::NextPage {
        request_page_key: "page".into(),
        next_page_pointer: "/nextPage".into(),
        has_more_pointer: "/hasMore".into(),
    };
    let cont = Some(serde_json::json!(0));
    let result = AlphaApiClient::build_qparams(&pagination, &cont);
    assert!(result.is_err(), "zero continuation must be rejected");
}

#[test]
fn build_qparams_rejects_negative_continuation() {
    let pagination = PaginationConfig::NextPage {
        request_page_key: "page".into(),
        next_page_pointer: "/nextPage".into(),
        has_more_pointer: "/hasMore".into(),
    };
    let cont = Some(serde_json::json!(-1));
    let result = AlphaApiClient::build_qparams(&pagination, &cont);
    assert!(result.is_err(), "negative continuation must be rejected");
}

#[test]
fn build_qparams_rejects_fractional_continuation() {
    let pagination = PaginationConfig::NextPage {
        request_page_key: "page".into(),
        next_page_pointer: "/nextPage".into(),
        has_more_pointer: "/hasMore".into(),
    };
    let cont = Some(serde_json::json!(1.5));
    let result = AlphaApiClient::build_qparams(&pagination, &cont);
    assert!(result.is_err(), "fractional continuation must be rejected");
}

#[test]
fn build_qparams_accepts_positive_integer_continuation() {
    let pagination = PaginationConfig::NextPage {
        request_page_key: "page".into(),
        next_page_pointer: "/nextPage".into(),
        has_more_pointer: "/hasMore".into(),
    };
    let cont = Some(serde_json::json!(1));
    let result = AlphaApiClient::build_qparams(&pagination, &cont);
    assert!(result.is_ok(), "positive integer continuation must be accepted");
    let params = result.unwrap();
    assert_eq!(params["page"], 1);
}


#[tokio::test(flavor = "multi_thread")]
async fn check_completeness_rejects_fractional_next_page() {
    let json = r#"{
        "groupedRankings": [], "page": 1, "complete": true, "continuation": null,
        "nextPage": 1.5, "hasMore": true
    }"#;
    let raw = RawRankingsResponse::from_json(json).unwrap();
    let client = make_full_pagination_config("http://example.com");
    let result = client.check_completeness(&raw);
    assert!(result.is_err(), "fractional nextPage must be rejected, got {:?}", result);
}

#[tokio::test(flavor = "multi_thread")]
async fn check_completeness_rejects_zero_next_page() {
    let json = r#"{
        "groupedRankings": [], "page": 1, "complete": true, "continuation": null,
        "nextPage": 0, "hasMore": true
    }"#;
    let raw = RawRankingsResponse::from_json(json).unwrap();
    let client = make_full_pagination_config("http://example.com");
    let result = client.check_completeness(&raw);
    assert!(result.is_err(), "zero nextPage must be rejected, got {:?}", result);
}

#[tokio::test(flavor = "multi_thread")]
async fn check_completeness_rejects_empty_object_next_page() {
    let json = r#"{
        "groupedRankings": [], "page": 1, "complete": true, "continuation": null,
        "nextPage": {}, "hasMore": true
    }"#;
    let raw = RawRankingsResponse::from_json(json).unwrap();
    let client = make_full_pagination_config("http://example.com");
    let result = client.check_completeness(&raw);
    assert!(result.is_err(), "empty object nextPage must be rejected, got {:?}", result);
}


#[tokio::test(flavor = "multi_thread")]
async fn check_completeness_accepts_valid_next_page_token() {
    let json = r#"{
        "groupedRankings": [[{"AthleteID":1,"AthleteName":"Test","GradeID":2,"TeamName":"School","State":"CA","Results":[{"MeetID":1,"MeetName":"State","IDResult":500,"EventShort":"100m","Measure":"10.55","ResultDate":"2026-06-15","SeasonID":2026}]}]],
        "page": 1, "complete": true, "continuation": null,
        "nextPage": "2", "hasMore": true
    }"#;
    let raw = RawRankingsResponse::from_json(json).unwrap();
    let client = make_full_pagination_config("http://example.com");
    let result = client.check_completeness(&raw);
    assert!(matches!(result, Ok(false)), "valid string nextPage with hasMore=true => incomplete");
}
// --- Cap markers constructor validation tests ---

fn make_client_with_cap_markers(cap_markers: Vec<String>) -> Result<AlphaApiClient, AlphaApiError> {
    AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "https://example.com".into(),
        rankings_path: "/api/v1/tfRankings/GetRankings".into(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".into(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse {
            complete_pointer: "/complete".into(),
        },
        allowed_routes: vec![
            "/api/v1/tfRankings/GetRankings".into(),
            "/api/v1/tfRankings/GetNavInfo".into(),
        ],
        allowed_fields: vec!["AthleteID".into(), "AthleteName".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers,
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true,
        permission_reference: "test-permission".into(),
    })
}

#[test]
fn new_rejects_empty_cap_marker() {
    let result = make_client_with_cap_markers(vec!["".into()]);
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn new_rejects_malformed_rfc6901_tilde_escape() {
    let result = make_client_with_cap_markers(vec!["/bad~2".into()]);
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn new_rejects_trailing_tilde_in_pointer() {
    let result = make_client_with_cap_markers(vec!["/path~".into()]);
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn new_accepts_valid_escaped_pointer_tilde0() {
    let result = make_client_with_cap_markers(vec!["/a~0b".into()]);
    assert!(result.is_ok());
}

#[test]
fn new_accepts_valid_escaped_pointer_tilde1() {
    let result = make_client_with_cap_markers(vec!["/path~1key".into()]);
    assert!(result.is_ok());
}

#[test]
fn new_accepts_top_level_key_without_special_chars() {
    let result = make_client_with_cap_markers(vec!["rankings".into()]);
    assert!(result.is_ok());
}

#[test]
fn new_rejects_top_level_key_containing_slash() {
    let result = make_client_with_cap_markers(vec!["a/b".into()]);
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn new_rejects_top_level_key_containing_tilde() {
    let result = make_client_with_cap_markers(vec!["a~b".into()]);
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn new_rejects_empty_and_malformed_markers_in_list() {
    let result = make_client_with_cap_markers(vec!["valid".into(), "".into(), "/bad~2".into()]);
    assert!(matches!(result, Err(AlphaApiError::InvalidConfig(_))));
}

#[test]
fn new_accepts_multiple_valid_markers() {
    let result = make_client_with_cap_markers(vec![
        "rankings".into(),
        "/data".into(),
        "/a~0b".into(),
        "/path~1key".into(),
    ]);
    assert!(result.is_ok());
}
