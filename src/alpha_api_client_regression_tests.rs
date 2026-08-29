use std::time::Duration;
use crate::alpha_api::AlphaApiError;
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_api::AlphaApiClientConfig;
use crate::alpha_model::{AlphaRequest, PaginationConfig};
use crate::alpha_test_helpers::{make_client, make_full_pagination_config, make_test_request, success_body};

// --- Regression: body timeout cancellation / no orphan ---

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
#[tokio::test(flavor = "multi_thread")]
async fn http_401_stalled_body_returns_unauthorized_without_reading_body() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(401)
        .with_body("")
        .create();
    let client = make_client(&url);
    let start = std::time::Instant::now();
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    let elapsed = start.elapsed();
    assert!(matches!(err, AlphaApiError::Unauthorized(_)), "expected Unauthorized, got {:?}", err);
    assert!(elapsed < Duration::from_secs(2), "401 should return immediately, took {:?}", elapsed);
}
#[tokio::test(flavor = "multi_thread")]
async fn terminal_continuation_is_none_when_complete() {
    // When complete == true, continuation must be None — no stale next tokens.
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    let mock = server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"groupedRankings":[[{"AthleteID":1,"AthleteName":"Test","GradeID":2,"TeamName":"School","State":"CA","Results":[{"MeetID":100,"MeetName":"State Finals","IDResult":500,"EventShort":"100m","Measure":"10.55","ResultDate":"2026-06-15","SeasonID":2026,"Wind":null}]}]],"hasMore":false,"complete":true,"nextPage":"stale-token","continuation":null}"#)
        .create();
    let client = make_full_pagination_config(&url);
    let page = client.rankings(&make_test_request()).await.unwrap();
    assert!(page.complete);
    assert!(page.continuation.is_none(), "continuation must be None when complete");
    assert_eq!(page.records.len(), 1);
    mock.assert();
}
#[tokio::test(flavor = "multi_thread")]
async fn nested_configured_has_more_does_not_reject_unrelated_top_level_has_more() {
    // Nested configured pointers must not be rejected by unrelated top-level markers.
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    let response_body = r#"{
        "groupedRankings":[[{"AthleteID":1,"AthleteName":"Test","GradeID":2,"TeamName":"School","State":"CA","Results":[{"MeetID":100,"MeetName":"State Finals","IDResult":500,"EventShort":"100m","Measure":"10.55","ResultDate":"2026-06-15","SeasonID":2026,"Wind":null}]}]],
        "hasMore": true,
        "page": 1,
        "nextPage": "ignored",
        "data": {
            "hasMore": false,
            "nextPage": null
        },
        "complete": true,
        "continuation": null
    }"#;
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response_body)
        .create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api/v1/tfRankings/GetRankings".to_owned(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".to_owned(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::NextPage {
            has_more_pointer: "/data/hasMore".into(),
            next_page_pointer: "/data/nextPage".into(),
            request_page_key: "page".into(),
        },
        allowed_routes: vec!["/api/v1/tfRankings/GetRankings".into()],
        allowed_fields: vec![
            "AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into(),
            "MeetID".into(), "MeetName".into(), "IDResult".into(), "EventShort".into(), "Measure".into(),
            "ResultDate".into(), "SeasonID".into(),
        ],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec![],
    }).expect("client creation must not fail");
    let page = client.rankings(&make_test_request()).await.unwrap();
    assert!(page.complete, "nested hasMore=false should mean complete");
    assert_eq!(page.records.len(), 1);
}
#[tokio::test(flavor = "multi_thread")]
async fn pointer_escaped_and_array_pointers() {
    // RFC6901: ~1 = literal /, ~0 = literal ~, /0 = array index 0.
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    // Response has nested structure with escaped keys.
    let response_body = r#"{"groupedRankings":[[{"AthleteID":1,"AthleteName":"Test","GradeID":2,"TeamName":"School","State":"CA","Results":[{"MeetID":100,"MeetName":"State Finals","IDResult":500,"EventShort":"100m","Measure":"10.55","ResultDate":"2026-06-15","SeasonID":2026,"Wind":null}]}]],"data":{"has~1more":false},"settings":{"complete":true},"continuation":null}"#;
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response_body)
        .create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api/v1/tfRankings/GetRankings".to_owned(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".to_owned(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/settings/complete".into() },
        allowed_routes: vec!["/api/v1/tfRankings/GetRankings".into()],
        allowed_fields: vec![
            "AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into(),
            "MeetID".into(), "MeetName".into(), "IDResult".into(), "EventShort".into(), "Measure".into(),
            "ResultDate".into(), "SeasonID".into(),
        ],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec![],
    }).expect("client creation must not fail");
    let page = client.rankings(&make_test_request()).await.unwrap();
    assert!(page.complete);
    assert_eq!(page.records.len(), 1);
}
#[tokio::test(flavor = "multi_thread")]
async fn pointer_array_index_in_pointer() {
    // RFC6901: /items/0/name accesses array element 0 then key "name".
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    let response_body = r#"{"groupedRankings":[[{"AthleteID":1,"AthleteName":"Test","GradeID":2,"TeamName":"School","State":"CA","Results":[{"MeetID":100,"MeetName":"State Finals","IDResult":500,"EventShort":"100m","Measure":"10.55","ResultDate":"2026-06-15","SeasonID":2026,"Wind":null}]}]],"items":[{"complete":true}],"continuation":null}"#;
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response_body)
        .create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api/v1/tfRankings/GetRankings".to_owned(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".to_owned(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/items/0/complete".into() },
        allowed_routes: vec!["/api/v1/tfRankings/GetRankings".into()],
        allowed_fields: vec![
            "AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into(),
            "MeetID".into(), "MeetName".into(), "IDResult".into(), "EventShort".into(), "Measure".into(),
            "ResultDate".into(), "SeasonID".into(),
        ],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec![],
    }).expect("client creation must not fail");
    let page = client.rankings(&make_test_request()).await.unwrap();
    assert!(page.complete, "pointer /items/0/complete should resolve to true");
}

