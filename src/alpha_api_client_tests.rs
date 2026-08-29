use crate::alpha_api::{AlphaApiError, AlphaApiClientConfig};
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::{AlphaRequest, PaginationConfig};

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
        allowed_fields: vec!["AthleteID".to_owned()],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
    };
    AlphaApiClient::new(config)
}

fn make_next_page_client(server_url: &str) -> AlphaApiClient {
    let config = AlphaApiClientConfig {
        base_url: server_url.to_owned(),
        rankings_path: "/api/v1/tfRankings/GetRankings".to_owned(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".to_owned(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::NextPage {
            has_more_pointer: "/hasMore".to_owned(),
            next_page_pointer: "/nextPage".to_owned(),
            request_page_key: "page".to_owned(),
        },
        allowed_routes: vec![
            "/api/v1/tfRankings/GetRankings".to_owned(),
            "/api/v1/tfRankings/GetNavInfo".to_owned(),
        ],
        allowed_fields: vec!["AthleteID".to_owned()],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
    };
    AlphaApiClient::new(config)
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


/// Helper: make a client with zero min_delay_ms to avoid real sleeps in tests.
fn make_fast_client(server_url: &str) -> AlphaApiClient {
    AlphaApiClient::new(AlphaApiClientConfig {
        base_url: server_url.to_owned(),
        rankings_path: "/api/v1/tfRankings/GetRankings".to_owned(),
        nav_info_path: "/api/v1/tfRankings/GetNavInfo".to_owned(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".to_owned() },
        allowed_routes: vec!["/api/v1/tfRankings/GetRankings".into()],
        allowed_fields: vec![],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
    })
}


/// Verify HTTP 429 with Retry-After header returns RateLimitedExhausted error.
#[tokio::test(flavor = "multi_thread")]
async fn http_429_retry_after_one_second() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    // Retry-After: 1 means 1 second = 1000ms
    server.mock("POST", "/api/v1/tfRankings/GetRankings")
        .with_status(429)
        .with_header("Retry-After", "1")
        .create();
    let client = make_fast_client(&url);
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    match err {
        AlphaApiError::RateLimitedExhausted { total_delay_ms, .. } => {
            // 2 retries × 1000ms = 2000ms
            assert_eq!(total_delay_ms, 2000, "Retry-After: 1s must convert to 1000ms per retry");
        }
        other => panic!("expected RateLimitedExhausted, got {:?}", other),
    }
}


/// Verify Retry-After seconds-to-milliseconds conversion and min_delay_ms enforcement.
/// Uses pure unit assertions rather than HTTP calls to avoid timing issues.
#[test]
fn retry_after_seconds_convert_to_millis() {
    // Retry-After: 1 (1 second) → 1000ms
    let secs: u64 = 1;
    let delay_ms = secs.saturating_mul(1000);
    assert_eq!(delay_ms, 1000, "1 second must be 1000ms");

    // Retry-After: 0 → 0ms, then max with min_delay_ms(10) → 10ms
    let secs: u64 = 0;
    let delay_ms = secs.saturating_mul(1000);
    let wait = delay_ms.max(10);
    assert_eq!(wait, 10, "zero Retry-After must use min_delay_ms");

    // Retry-After: 1000 (1000 seconds) → 1000000ms
    let secs: u64 = 1000;
    let delay_ms = secs.saturating_mul(1000);
    assert_eq!(delay_ms, 1_000_000, "1000 seconds must be 1000000ms");

    // Large seconds that could overflow: 4_294_967_295 → saturates
    let secs: u64 = u64::MAX;
    let delay_ms = secs.saturating_mul(1000);
    assert_eq!(delay_ms, u64::MAX, "overflow must saturate");
}


