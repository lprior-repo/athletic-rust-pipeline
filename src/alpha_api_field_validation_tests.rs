use crate::alpha_api::AlphaApiClientConfig;
use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::PaginationConfig;
use crate::alpha_model_raw::{RawRankingsResponse, RawRankingRecord};

// --- Malformed row rejection ---
#[test]
fn malformed_nested_row_rejected_via_from_json() {
    // Malformed nested result (missing IDResult) causes from_json to reject the entire response.
    let json = r#"{
        "groupedRankings": [[{
            "AthleteID": 1,
            "AthleteName": "Test",
            "GradeID": 2,
            "TeamName": "School",
            "State": "CA",
            "Results": [{
                "MeetID": 100,
                "MeetName": "Meet",
                "EventShort": "100m",
                "Measure": "10.5",
                "ResultDate": "2026-06-15",
                "SeasonID": 2026
            }]
        }]],
        "page": 1
    }"#;
    // from_json propagates errors — malformed row causes rejection
    let result = RawRankingsResponse::from_json(json);
    assert!(result.is_err(), "malformed nested row must reject the entire response");
}
#[test]
fn valid_nested_row_succeeds() {
    let json = r#"{
        "groupedRankings": [[{
            "AthleteID": 1,
            "AthleteName": "Test",
            "GradeID": 2,
            "TeamName": "School",
            "State": "CA",
            "Results": [{
                "MeetID": 100,
                "MeetName": "Meet",
                "IDResult": 500,
                "EventShort": "100m",
                "Measure": "10.5",
                "ResultDate": "2026-06-15",
                "SeasonID": 2026
            }]
        }]],
        "page": 1
    }"#;
    let raw = RawRankingsResponse::from_json(json).unwrap();
    assert_eq!(raw.grouped_rankings[0].len(), 1);
}
#[test]
fn flattened_row_missing_required_returns_error() {
    let json = r#"{
        "groupedRankings": [[{
            "AthleteID": 1,
            "AthleteName": "Test",
            "GradeID": 2,
            "TeamName": "School",
            "State": "CA",
            "EventShort": "100m",
            "Measure": "10.5"
        }]]
    }"#;
    let raw = RawRankingsResponse::from_json(json).unwrap();
    // Missing IDResult, ResultDate, SeasonID => error
    assert!(raw.grouped_rankings[0][0].to_flattened_records().is_err());
}
#[test]
fn unknown_fields_in_ranking_record_silently_ignored() {
    // Unknown fields are silently ignored (no deny_unknown_fields).
    let json = r#"{
        "AthleteID": 1,
        "AthleteName": "Test",
        "GradeID": 2,
        "TeamName": "School",
        "State": "CA",
        "UnknownField": "ignored",
        "AnotherUnknown": 42
    }"#;
    let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
    assert_eq!(rec.athlete_id, 1);
    assert_eq!(rec.athlete_name, "Test");
}

// --- Field enforcement ---
#[test]
fn enforce_allowed_fields_filters() {
    let client = AlphaApiClient::new(AlphaApiClientConfig {
        base_url: "https://example.com".to_owned(),
        rankings_path: "/rankings".to_owned(),
        nav_info_path: "/nav".to_owned(),
        timeout_seconds: 30,
        max_retries: 2,
        pagination: PaginationConfig::SingleResponse { complete_pointer: "/complete".to_owned() },
        allowed_routes: vec![],
        allowed_fields: vec![
            "AthleteID".into(), "AthleteName".into(), "GradeID".into(), "TeamName".into(),
            "State".into(), "MeetID".into(), "MeetName".into(), "IDResult".into(),
            "EventShort".into(), "Measure".into(), "ResultDate".into(), "SeasonID".into(),
            "Wind".into(), "unknown".into(),
        ],
        max_concurrent_requests: 1,
        min_delay_ms: 0,
        cap_markers: vec![],
    }).unwrap();
    let input = serde_json::json!({
        "groupedRankings": [[{
            "AthleteID": 1, "AthleteName": "Test", "GradeID": 2, "TeamName": "T",
            "State": "CA", "MeetID": 3, "MeetName": "M", "IDResult": 4,
            "EventShort": "100m", "Measure": "10.5", "ResultDate": "2024-01-01",
            "SeasonID": 5, "Wind": "1.2", "unknown_field": "remove_me"
        }]]
    });
    let result = client.enforce_response_allowed_fields(input).unwrap();
    let groups = result.get("groupedRankings").unwrap().as_array().unwrap();
    let rec = groups[0].as_array().unwrap()[0].as_object().unwrap();
    assert!(rec.contains_key("AthleteID"));
    assert!(rec.contains_key("MeetName"));
    assert!(!rec.contains_key("unknown_field"), "unknown fields must be stripped");
}
#[test]
fn serialize_rankings_body_qparams_with_continuation() {
    let pagination = PaginationConfig::NextPage {
        has_more_pointer: "/hasMore".to_owned(),
        next_page_pointer: "/nextPage".to_owned(),
        request_page_key: "page".to_owned(),
    };
    let continuation = Some(serde_json::json!({"page": "next_1"}));
    let qparams = AlphaApiClient::build_qparams(&pagination, &continuation);
    assert_eq!(qparams["page"], serde_json::json!({"page": "next_1"}));
}
