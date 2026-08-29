use crate::alpha_api::{AlphaApiError, AlphaApiClientConfig};
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::{AlphaRequest, PaginationConfig};
use crate::alpha_test_helpers::{make_client, make_full_pagination_config, make_test_request, success_body};
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

#[tokio::test(flavor = "multi_thread")]
async fn http_429_wait_maxes_retry_after_against_min_delay() {
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
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api/v1/tfRankings/GetRankings".to_owned(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".to_owned(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".to_owned() },
        allowed_routes: vec!["/api/v1/tfRankings/GetRankings".into()],
        allowed_fields: vec![
            "AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into(),
            "MeetID".into(), "MeetName".into(), "IDResult".into(), "EventShort".into(), "Measure".into(),
            "ResultDate".into(), "SeasonID".into(),
        ],
        max_concurrent_requests: 1,
        min_delay_ms: 500,
        cap_markers: vec![],
    }).expect("client creation must not fail");
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    match err {
        AlphaApiError::RateLimitedExhausted { total_delay_ms, .. } => {
            assert_eq!(total_delay_ms, 1000, "must use max(Retry-After, min_delay_ms) * attempt");
        }
        other => panic!("expected RateLimitedExhausted, got {:?}", other),
    }
}
#[tokio::test(flavor = "multi_thread")]
async fn request_boundary_single_response_has_exact_protocol_keys() {
    // Finding 4: assert the production request body has exactly the ten
    // protocol keys including qParams, numeric divListId, and empty qParams.
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    let mock = server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .match_body(mockito::Matcher::Json(serde_json::json!({
            "reportType": "div", "mode": "list", "divListId": 12,
            "indoor": false, "eventShort": "100m", "gender": "m",
            "qualifyingListKey": "", "version": 2, "debug": "",
            "qParams": serde_json::json!({}),
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(success_body())
        .create();
    let client = make_client(&url);
    let page = client.rankings(&make_test_request()).await.unwrap();
    assert!(page.complete);
    assert_eq!(page.records.len(), 1);
    mock.assert();
}

#[tokio::test(flavor = "multi_thread")]
async fn request_boundary_next_page_has_qparams_with_continuation() {
    // Finding 4: NextPage mode — qParams contains the continuation token.
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    let cont_token = serde_json::json!("next-page-42");
    let mock = server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .match_body(mockito::Matcher::Json(serde_json::json!({
            "reportType": "div", "mode": "list", "divListId": 12,
            "indoor": false, "eventShort": "100m", "gender": "m",
            "qualifyingListKey": "", "version": 2, "debug": "",
            "qParams": serde_json::json!({ "page": cont_token }),
        })))
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"groupedRankings":[[{"AthleteID":1,"AthleteName":"Test","GradeID":2,"TeamName":"School","State":"CA","Results":[{"MeetID":100,"MeetName":"State Finals","IDResult":500,"EventShort":"100m","Measure":"10.55","ResultDate":"2026-06-15","SeasonID":2026,"Wind":null}]}]],"page":1,"hasMore":false,"complete":true,"nextPage":null,"continuation":null}"#)
        .create();
    let req = AlphaRequest {
        state_id: 12, season_id: 2026, gender: "m".into(),
        event_short: "100m".into(), indoor: false,
        continuation: Some(cont_token),
    };
    let client = make_full_pagination_config(&url);
    let page = client.rankings(&req).await.unwrap();
    assert!(page.complete);
    assert_eq!(page.records.len(), 1);
    mock.assert();
}
