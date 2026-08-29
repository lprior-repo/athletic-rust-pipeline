use crate::alpha_api::{AlphaApiError, AlphaApiClientConfig};
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::{AlphaRequest, PaginationConfig};

fn make_test_request() -> AlphaRequest {
    AlphaRequest {
        state_id: 1,
        season_id: 2024,
        gender: "Female".to_owned(),
        event_short: "100m".to_owned(),
        indoor: false,
        continuation: None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn nextpage_cap_marker_rejected() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("POST", "/rankings")
        .match_body(mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "page": 1,
            "complete": false,
            "continuation": null,
            "groupedRankings": [[{"AthleteID": 1, "AthleteName": "Test", "GradeID": 1, "TeamName": "", "State": ""}]],
            "hasMore": true,
            "nextPage": "next-token",
            "__cap": true
        }"#)
        .create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/rankings".to_owned(),
        nav_info_path: "/nav".to_owned(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::NextPage {
            has_more_pointer: "/hasMore".to_owned(),
            next_page_pointer: "/nextPage".to_owned(),
            request_page_key: "page".to_owned(),
        },
        allowed_routes: vec!["/rankings".to_owned()],
        allowed_fields: vec![],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec!["__cap".into()],
    }).expect("client must not fail");
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::TruncatedWithoutContinuation));
}

#[tokio::test(flavor = "multi_thread")]
async fn single_response_cap_marker_rejected() {
    let (mut server, url) = tokio::task::spawn_blocking(|| {
        let server = mockito::Server::new();
        let url = server.url();
        (server, url)
    }).await.unwrap();
    server.mock("POST", "/api")
        .match_body(mockito::Matcher::Any)
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{
            "page": 1,
            "complete": true,
            "continuation": null,
            "groupedRankings": [[{"AthleteID": 1, "AthleteName": "Test", "GradeID": 1, "TeamName": "", "State": ""}]],
            "__cap": true
        }"#)
        .create();
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: url,
        rankings_path: "/api".to_owned(),
        nav_info_path: "/nav".to_owned(),
        timeout_seconds: 10,
        max_retries: 0,
        pagination: PaginationConfig::SingleResponse {
            complete_pointer: "/complete".to_owned(),
        },
        allowed_routes: vec!["/api".to_owned()],
        allowed_fields: vec!["AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec!["__cap".into()],
    }).expect("client must not fail");
    let err = client.rankings(&make_test_request()).await.unwrap_err();
    assert!(matches!(err, AlphaApiError::TruncatedWithoutContinuation));
}

#[test]
fn enforce_allowed_fields_removes_nested_results_disallowed() {
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".to_owned(),
        rankings_path: "/api".to_owned(),
        nav_info_path: "/nav".to_owned(),
        timeout_seconds: 10,
        max_retries: 0,
        pagination: PaginationConfig::SingleResponse {
            complete_pointer: "/complete".to_owned(),
        },
        allowed_routes: vec!["/api".to_owned()],
        allowed_fields: vec![
            "AthleteID".into(), "AthleteName".into(), "GradeID".into(),
            "TeamName".into(), "State".into(), "Results".into(),
            "IDResult".into(), "EventShort".into(),
        ],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec![],
    }).expect("client must not fail");
    let json = r#"{
        "page": 1,
        "complete": false,
        "groupedRankings": [[{
            "AthleteID": 1,
            "AthleteName": "Test",
            "GradeID": 1,
            "TeamName": "",
            "State": "",
            "Results": [{
                "IDResult": 100,
                "EventShort": "100m",
                "Measure": "10.50",
                "ResultDate": "2024-01-01",
                "SeasonID": 2024,
                "MeetID": 500,
                "MeetName": "Meet",
                "Wind": "+0.5"
            }]
        }]]
    }"#;
    let value: serde_json::Value = serde_json::from_str(json).unwrap();
    let filtered = client.enforce_response_allowed_fields(value).unwrap();
    assert!(filtered.get("page").is_some());
    assert!(filtered.get("complete").is_some());
    let recs = filtered["groupedRankings"][0].as_array().unwrap();
    let rec = &recs[0];
    let results = rec["Results"].as_array().unwrap();
    let result = &results[0];
    assert!(result.get("IDResult").is_some());
    assert!(result.get("EventShort").is_some());
    assert!(result.get("Measure").is_none());
    assert!(result.get("ResultDate").is_none());
    assert!(result.get("SeasonID").is_none());
    assert!(result.get("MeetID").is_none());
    assert!(result.get("MeetName").is_none());
    assert!(result.get("Wind").is_none());
}

#[test]
fn enforce_allowed_fields_preserves_envelope() {
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "http://example.com".to_owned(),
        rankings_path: "/api".to_owned(),
        nav_info_path: "/nav".to_owned(),
        timeout_seconds: 10,
        max_retries: 0,
        pagination: PaginationConfig::SingleResponse {
            complete_pointer: "/complete".to_owned(),
        },
        allowed_routes: vec!["/api".to_owned()],
        allowed_fields: vec!["AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(), "State".into()],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec![],
    }).expect("client must not fail");
    let json = r#"{
        "page": 1,
        "complete": false,
        "continuation": "token",
        "groupedRankings": [[{"AthleteID": 1, "AthleteName": "Test", "GradeID": 1, "TeamName": "", "State": ""}]]
    }"#;
    let value: serde_json::Value = serde_json::from_str(json).unwrap();
    let filtered = client.enforce_response_allowed_fields(value).unwrap();
    assert!(filtered.get("page").is_some());
    assert!(filtered.get("complete").is_some());
    assert!(filtered.get("continuation").is_some());
    assert!(filtered.get("groupedRankings").is_some());
}