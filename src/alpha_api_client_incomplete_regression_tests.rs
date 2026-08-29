use crate::alpha_api::AlphaApiError;
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_api::AlphaApiClientConfig;
use crate::alpha_model::PaginationConfig;
use crate::alpha_test_helpers::make_test_request;

// --- Regression: SingleResponse incomplete fails closed ---

#[tokio::test(flavor = "multi_thread")]
async fn single_response_incomplete_fails_closed() {
    // SingleResponse mode with continuation.complete=false but valid nextPage
    // must fail with Incomplete, not Ok(false) with continuation=None.
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"groupedRankings":[[{"AthleteID":1,"AthleteName":"Test","GradeID":2,"TeamName":"School","State":"CA","Results":[{"MeetID":100,"MeetName":"State Finals","IDResult":500,"EventShort":"100m","Measure":"10.55","ResultDate":"2026-06-15","SeasonID":2026,"Wind":null}]}]],"hasMore":true,"nextPage":"token-42","settings":{"complete":false},"continuation":{"page":0,"complete":false}}"#)
        .create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api/v1/tfRankings/GetRankings".into(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".into(),
        timeout_seconds: 30,
        max_retries: 0,
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
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::Incomplete(_)), "SingleResponse incomplete must fail closed, got {:?}", err);
}

// --- Regression: cap_markers JSON-pointer paths ---

#[tokio::test(flavor = "multi_thread")]
async fn cap_markers_json_pointer_path() {
    // Cap marker with / prefix uses value.pointer(), checking nested metadata.
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"groupedRankings":[],"page":1,"complete":true,"continuation":null,"metadata":{"truncated":true}}"#)
        .create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api/v1/tfRankings/GetRankings".into(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".into(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api/v1/tfRankings/GetRankings".into()],
        allowed_fields: vec![
            "AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into(),
            "MeetID".into(), "MeetName".into(), "IDResult".into(), "EventShort".into(), "Measure".into(),
            "ResultDate".into(), "SeasonID".into(),
        ],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec!["/metadata/truncated".into()],
    }).expect("client creation must not fail");
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::TruncatedWithoutContinuation), "JSON-pointer cap marker must detect truncation");
}

#[tokio::test(flavor = "multi_thread")]
async fn cap_markers_top_level_key_path() {
    // Cap marker without / prefix uses value.get(), checking top-level key.
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"groupedRankings":[],"page":1,"complete":true,"continuation":null,"__cap":true}"#)
        .create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api/v1/tfRankings/GetRankings".into(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".into(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api/v1/tfRankings/GetRankings".into()],
        allowed_fields: vec![
            "AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into(),
            "MeetID".into(), "MeetName".into(), "IDResult".into(), "EventShort".into(), "Measure".into(),
            "ResultDate".into(), "SeasonID".into(),
        ],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec!["__cap".into()],
    }).expect("client creation must not fail");
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::TruncatedWithoutContinuation), "top-level cap marker must detect truncation");
}

#[tokio::test(flavor = "multi_thread")]
async fn single_response_has_more_true_valid_next_page_returns_incomplete() {
    // SingleResponse hasMore=true with valid nextPage must return Incomplete.
    // SingleResponse cannot produce a continuation token, so this is non-resumable.
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"groupedRankings":[],"hasMore":true,"nextPage":"token-42"}"#)
        .create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api/v1/tfRankings/GetRankings".into(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".into(),
        timeout_seconds: 30,
        max_retries: 0,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
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
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::Incomplete(_)), "SingleResponse hasMore=true with valid nextPage must return Incomplete, got {:?}", err);
}
#[tokio::test(flavor = "multi_thread")]
async fn cap_marker_wrong_type_returns_truncated_fail_closed() {
    // Cap marker with wrong type (string) must be treated as truncated, not absent.
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"groupedRankings":[],"truncated":"yes"}"#)
        .create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api/v1/tfRankings/GetRankings".into(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".into(),
        timeout_seconds: 30,
        max_retries: 0,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".into() },
        allowed_routes: vec!["/api/v1/tfRankings/GetRankings".into()],
        allowed_fields: vec![
            "AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into(),
            "MeetID".into(), "MeetName".into(), "IDResult".into(), "EventShort".into(), "Measure".into(),
            "ResultDate".into(), "SeasonID".into(),
        ],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec!["truncated".into()],
    }).expect("client creation must not fail");
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::TruncatedWithoutContinuation), "wrong-type cap marker must fail-closed as truncated");
}
