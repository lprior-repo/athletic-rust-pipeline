use crate::alpha_api::{AlphaApiError, AlphaApiClientConfig};
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::PaginationConfig;
#[tokio::test(flavor = "multi_thread")]
async fn nav_info_rejects_empty_response() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("GET", "/api/v1/tfRankings/GetNavInfo")
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
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true,
        permission_reference: "test".into(),
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
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true,
        permission_reference: "test".into(),
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
        .with_body(r#"{"state": {"StateID": 1, "State": "CA", "StateName": "California"}, "event": {"EventShort": "100m", "EventName": "100 Meters"}, "divisions": [{"DivisionID": 1, "DivisionName": "Div 1", "Indoor": false}], "genders": ["m", "f"], "complete": true, "page": 1}"#)
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
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true,
        permission_reference: "test".into(),
    }).expect("client must not fail");

    let result = client.nav_info(2024, false).await;
    assert!(result.is_ok(), "partial response with complete must succeed");
}
