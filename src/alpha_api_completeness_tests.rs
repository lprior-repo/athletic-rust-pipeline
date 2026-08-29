use crate::alpha_api::{AlphaApiError, AlphaApiClientConfig};
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::{PaginationConfig, RawRankingsResponse};

fn make_single_response_client(server_url: &str, pointer: &str) -> AlphaApiClient {
    AlphaApiClient::new(AlphaApiClientConfig {
        base_url: server_url.to_owned(),
        rankings_path: "/rankings".to_owned(),
        nav_info_path: "/nav".to_owned(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: pointer.to_owned() },
        allowed_routes: vec![],
        allowed_fields: vec![],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
    })
}

fn make_next_page_client(server_url: &str) -> AlphaApiClient {
    AlphaApiClient::new(AlphaApiClientConfig {
        base_url: server_url.to_owned(),
        rankings_path: "/rankings".to_owned(),
        nav_info_path: "/nav".to_owned(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::NextPage {
            has_more_pointer: "/hasMore".to_owned(),
            next_page_pointer: "/nextPage".to_owned(),
            request_page_key: "page".to_owned(),
        },
        allowed_routes: vec![],
        allowed_fields: vec![],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
    })
}

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
        min_delay_ms: 0,
    })
}

// --- SingleResponse completeness ---

#[test]
fn single_response_complete_pointer_true() {
    let client = make_single_response_client("https://example.com", "/complete");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "complete": true}"#).unwrap();
    assert!(client.check_completeness(&raw));
}

#[test]
fn single_response_complete_pointer_false() {
    let client = make_single_response_client("https://example.com", "/complete");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "complete": false}"#).unwrap();
    assert!(!client.check_completeness(&raw));
}

#[test]
fn single_response_complete_pointer_missing() {
    let client = make_single_response_client("https://example.com", "/complete");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": []}"#).unwrap();
    assert!(!client.check_completeness(&raw));
}

#[test]
fn single_response_complete_pointer_wrong_type() {
    let client = make_single_response_client("https://example.com", "/complete");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "complete": "yes"}"#).unwrap();
    assert!(!client.check_completeness(&raw));
}

#[test]
fn single_response_complete_with_unknown_field() {
    let client = make_single_response_client("https://example.com", "/complete");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "complete": true, "unknown": 42}"#).unwrap();
    assert!(client.check_completeness(&raw));
}

#[test]
fn single_response_complete_with_nested_pointer() {
    let client = make_single_response_client("https://example.com", "/settings/complete");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "settings": {"complete": true}}"#).unwrap();
    assert!(client.check_completeness(&raw));
}

// --- NextPage completeness ---

#[test]
fn nextpage_complete_when_has_more_false() {
    let client = make_next_page_client("https://example.com");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "hasMore": false}"#).unwrap();
    assert!(client.check_completeness(&raw), "has_more=false means complete");
}

#[test]
fn nextpage_incomplete_when_has_more_true_no_next() {
    let client = make_next_page_client("https://example.com");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "hasMore": true, "nextPage": null}"#).unwrap();
    assert!(!client.check_completeness(&raw));
}

#[test]
fn nextpage_incomplete_when_has_more_true_empty_next() {
    let client = make_next_page_client("https://example.com");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "hasMore": true, "nextPage": ""}"#).unwrap();
    assert!(!client.check_completeness(&raw));
}

#[test]
fn nextpage_incomplete_when_has_more_true_valid_next() {
    let client = make_next_page_client("https://example.com");
    let raw = RawRankingsResponse::from_json(r#"{"groupedRankings": [], "hasMore": true, "nextPage": "2"}"#).unwrap();
    assert!(!client.check_completeness(&raw), "has_more=true with valid next page = incomplete");
}

#[test]
fn nextpage_complete_with_continuation_complete_false() {
    let client = make_next_page_client("https://example.com");
    let raw = RawRankingsResponse::from_json(r#"{ "groupedRankings": [], "hasMore": false, "continuation": {"page": 1, "complete": false} }"#).unwrap();
    assert!(!client.check_completeness(&raw), "continuation.complete=false forces incomplete");
}

// --- JSON pointer navigation ---

#[test]
fn json_pointer_walk_nested() {
    let value = serde_json::json!({ "groupedRankings": [[{"AthleteID": 1}]], "page": 1, "complete": true });
    let val = AlphaApiClient::walk_pointer_value(&value, "/complete");
    assert_eq!(val, Some(&serde_json::json!(true)));
}

#[test]
fn json_pointer_walk_missing() {
    let value = serde_json::json!({ "groupedRankings": [], "page": 1 });
    let val = AlphaApiClient::walk_pointer_value(&value, "/nonexistent");
    assert!(val.is_none());
}

#[test]
fn json_pointer_walk_array() {
    let value = serde_json::json!({ "results": [{"id": 1}, {"id": 2}, {"id": 3}] });
    let val = AlphaApiClient::walk_pointer_value(&value, "/results/1/id");
    assert_eq!(val, Some(&serde_json::json!(2)));
}

#[test]
fn json_pointer_walk_escaped_key() {
    let value = serde_json::json!({ "a~b/c": "value" });
    let val = AlphaApiClient::walk_pointer_value(&value, "/a~0b~1c");
    assert_eq!(val, Some(&serde_json::json!("value")));
}

// --- Nav info HTTP tests ---

#[tokio::test(flavor = "multi_thread")]
async fn nav_info_success() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("GET", "/api/v1/tfRankings/GetNavInfo")
        .match_query(mockito::Matcher::Any).with_status(200)
        .with_body(r#"{"state": {"StateID": 1, "State": "CA", "StateName": "California"}, "complete": true}"#)
        .create();
    let client = make_nav_info_client(&url);
    let resp = client.nav_info(2026, false).await.expect("nav_info should succeed");
    assert_eq!(resp.state.unwrap().StateID, Some(1));
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
        .with_status(200).with_body(r#"{"complete": true}"#).create();
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
        .with_status(429).with_header("Retry-After", "50").create();
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
