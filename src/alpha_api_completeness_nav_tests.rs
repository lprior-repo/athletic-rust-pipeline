use crate::alpha_api::{AlphaApiError, AlphaApiClientConfig};
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::PaginationConfig;

fn make_nav_info_client(server_url: &str) -> AlphaApiClient {
    AlphaApiClient::new(AlphaApiClientConfig {
        base_url: server_url.to_owned(),
        rankings_path: "/rankings".to_owned(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".to_owned(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".to_owned() },
        allowed_routes: vec!["/api/v1/tfRankings/GetNavInfo".into(), "/rankings".into()],
        allowed_fields: vec![],
        max_concurrent_requests: 1,
        min_delay_ms: 0, max_retry_delay_ms: 30_000,
        cap_markers: vec![],
        max_body_bytes: 8 * 1024 * 1024,
        auth_enabled: true,
        permission_reference: "test".into(),
    })
    .expect("client creation must not fail")
}

#[tokio::test(flavor = "multi_thread")]
async fn nav_info_success() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("GET", "/api/v1/tfRankings/GetNavInfo")
        .match_query(mockito::Matcher::Any).with_status(200)
        .with_body(r#"{"state": {"StateID": 1, "State": "CA", "StateName": "California"}, "event": {"EventShort": "100m", "EventName": "100 Meters"}, "divisions": [{"DivisionID": 1, "DivisionName": "Div 1", "Indoor": false}], "genders": ["m", "f"], "complete": true}"#)
        .create();
    let client = make_nav_info_client(&url);
    let resp = client.nav_info(2026, false).await.expect("nav_info should succeed");
    assert_eq!(resp.state.unwrap().state_id, Some(1));
}

#[tokio::test(flavor = "multi_thread")]
async fn nav_info_5xx_bounded_retry() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("GET", "/api/v1/tfRankings/GetNavInfo").with_status(500).create();
    server.mock("GET", "/api/v1/tfRankings/GetNavInfo").with_status(500).create();
    server.mock("GET", "/api/v1/tfRankings/GetNavInfo").match_query(mockito::Matcher::Any)
        .with_body(r#"{"state": {"StateID": 1, "State": "CA", "StateName": "California"}, "event": {"EventShort": "100m", "EventName": "100 Meters"}, "divisions": [{"DivisionID": 1, "DivisionName": "Div 1", "Indoor": false}], "genders": ["m", "f"], "complete": true}"#).create();
    let client = make_nav_info_client(&url);
    let resp = client.nav_info(2026, false).await.unwrap();
    assert_eq!(resp.complete, Some(true));
}

#[tokio::test(flavor = "multi_thread")]
async fn nav_info_429_with_retry_after() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("GET", "/api/v1/tfRankings/GetNavInfo").match_query(mockito::Matcher::Any)
        .with_status(429).with_header("Retry-After", "0").create();
    let client = make_nav_info_client(&url);
    let err = client.nav_info(2026, false).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::RateLimitedExhausted { .. }));
}

#[tokio::test(flavor = "multi_thread")]
async fn nav_info_non_2xx_rejected() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("GET", "/api/v1/tfRankings/GetNavInfo").match_query(mockito::Matcher::Any)
        .with_status(404).with_body("not found").create();
    let client = make_nav_info_client(&url);
    let err = client.nav_info(2026, false).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::UnexpectedStatus { status: 404, .. }));
}

#[tokio::test(flavor = "multi_thread")]
async fn nav_info_5xx_exhausted() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    for _ in 0..3 { server.mock("GET", "/api/v1/tfRankings/GetNavInfo").with_status(503).create(); }
    let client = make_nav_info_client(&url);
    let err = client.nav_info(2026, false).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::ServerErrorExhausted { .. }));
}
#[tokio::test(flavor = "multi_thread")]
async fn nav_info_rejects_empty_object() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("GET", "/api/v1/tfRankings/GetNavInfo")
        .match_query(mockito::Matcher::Any)
        .with_status(200)
        .with_body("{}")
        .create();
    let client = make_nav_info_client(&url);
    let err = client.nav_info(2026, false).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::Incomplete(_)), "bare object must be rejected");
}