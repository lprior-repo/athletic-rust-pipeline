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
    }).expect("client must not fail");
    let page = client.rankings(&make_test_request()).await.unwrap();
    assert!(page.continuation.is_none(), "SingleResponse must return None continuation");
}
