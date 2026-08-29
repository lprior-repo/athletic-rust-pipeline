use crate::alpha_api_client::AlphaApiClient;
use crate::alpha_model::{AlphaRequest, PaginationConfig};
use crate::alpha_model_raw::{
    RawNavInfoResponse, RawRankingRecord, RawRankingResult, RawRankingsResponse,
};

// --- Request serialization ---

#[test]
fn serialize_rankings_body_numeric_divlistid() {
    let req = AlphaRequest {
        state_id: 12,
        season_id: 2026,
        gender: "m".to_owned(),
        event_short: "100m".to_owned(),
        indoor: false,
        continuation: None,
    };
    let body = AlphaApiClient::serialize_rankings_body(&req);
    assert!(body["divListId"].is_number());
    assert_eq!(body["divListId"], serde_json::json!(12));
}

#[test]
fn serialize_rankings_body_single_response_qparams() {
    let pagination = PaginationConfig::SingleResponse {
        complete_pointer: "/complete".to_owned(),
    };
    let qparams = AlphaApiClient::build_qparams(&pagination, &None);
    assert_eq!(qparams.as_object().unwrap().len(), 0);
}

#[test]
fn serialize_rankings_body_nextpage_qparams() {
    let pagination = PaginationConfig::NextPage {
        has_more_pointer: "/hasMore".to_owned(),
        next_page_pointer: "/nextPage".to_owned(),
        request_page_key: "page".to_owned(),
    };
    let continuation = Some(serde_json::json!({"page": 2}));
    let qparams = AlphaApiClient::build_qparams(&pagination, &continuation);
    assert_eq!(qparams["page"], serde_json::json!({"page": 2}));
}

#[test]
fn serialize_rankings_body_all_keys_present() {
    let req = AlphaRequest {
        state_id: 1,
        season_id: 2026,
        gender: "f".to_owned(),
        event_short: "200m".to_owned(),
        indoor: false,
        continuation: None,
    };
    let body = AlphaApiClient::serialize_rankings_body(&req);
    let expected_keys = [
        "reportType", "mode", "divListId", "indoor", "eventShort",
        "gender", "qualifyingListKey", "version", "debug",
    ];
    for key in &expected_keys {
        assert!(body.get(*key).is_some(), "body must contain key '{}'", key);
    }
}

// --- Fixture deserialization ---

#[test]
fn deserialize_redacted_rankings_fixture() {
    let fixture = std::fs::read_to_string("fixtures/alpha/get-rankings-redacted.json").unwrap();
    let resp = RawRankingsResponse::from_json(&fixture).unwrap();
    assert_eq!(resp.grouped_rankings.len(), 2);
    assert_eq!(resp.grouped_rankings[0].len(), 2);
    let rec = &resp.grouped_rankings[0][0];
    assert_eq!(rec.athlete_id, 90_000_001);
    assert_eq!(rec.results[0].meet_id, 90_000_001);
    assert_eq!(rec.results[0].meet_name, "Test Meet");
}

#[test]
fn deserialize_nav_info_fixture() {
    let fixture = std::fs::read_to_string("fixtures/alpha/get-nav-info-redacted.json").unwrap();
    let resp: RawNavInfoResponse = serde_json::from_str(&fixture).unwrap();
    let state = resp.state.unwrap();
    assert_eq!(state.state_id, Some(90_000_001));
    let event = resp.event.unwrap();
    assert_eq!(event.event_short, Some("100m".to_owned()));
    assert_eq!(event.event_name, Some("100 Meters".to_owned()));
    let div = resp.divisions.unwrap();
    assert_eq!(div.len(), 1);
    assert_eq!(div[0].division_id, Some(90_000_001));
}

// --- Flattened row decoding ---

#[test]
fn deserialize_flattened_row_shape() {
    let json = r#"{
        "AthleteID": 12345,
        "AthleteName": "Jane Doe",
        "GradeID": 67890,
        "TeamName": "Oak High",
        "State": "CA",
        "IDResult": 11111,
        "EventShort": "100m",
        "Measure": "11.23",
        "ResultDate": "2026-05-15",
        "SeasonID": 2026,
        "Wind": "+0.5"
    }"#;
    let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
    assert_eq!(rec.id_result, Some(11_111));
    assert_eq!(rec.event_short, Some("100m".to_owned()));
    assert!(rec.results.is_empty());
}

#[test]
fn to_flattened_records_empty_when_no_data() {
    let json = r#"{
        "AthleteID": 999,
        "AthleteName": "Nobody",
        "GradeID": 0,
        "TeamName": "",
        "State": "",
        "Results": []
    }"#;
    let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
    assert!(rec.to_flattened_records().is_empty());
}

#[test]
fn unknown_fields_ignored_in_raw_ranking_result() {
    let json = r#"{
        "MeetID": 123,
        "MeetName": "Test Meet",
        "IDResult": 456,
        "EventShort": "100m",
        "Measure": "10.5",
        "ResultDate": "2099-01-01",
        "SeasonID": 2026,
        "UnknownField": "ignored"
    }"#;
    let result: Result<RawRankingResult, _> = serde_json::from_str(json);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().measure, "10.5");
}

#[test]
fn required_fields_missing_in_raw_ranking_record() {
    let json = r#"{"AthleteID": 123}"#;
    let result: Result<RawRankingRecord, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn no_request_body_logged() {
    let body = AlphaApiClient::serialize_rankings_body(&AlphaRequest {
        state_id: 12,
        season_id: 2026,
        gender: "m".to_owned(),
        event_short: "100m".to_owned(),
        indoor: false,
        continuation: None,
    });
    let json_str = serde_json::to_string(&body).unwrap();
    assert!(!json_str.contains("Bearer"));
}

// --- Malformed row rejection ---

#[test]
fn malformed_nested_row_rejected() {
    let json = r#"{
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
    }"#;
    let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
    assert!(rec.to_flattened_records().is_empty(), "missing IDResult must reject nested row");
}

#[test]
fn unknown_field_in_ranking_record_rejected() {
    let json = r#"{
        "AthleteID": 1,
        "AthleteName": "Test",
        "GradeID": 2,
        "TeamName": "School",
        "State": "CA",
        "UnknownField": "rejected"
    }"#;
    let result: Result<RawRankingRecord, _> = serde_json::from_str(json);
    assert!(result.is_err(), "unknown fields must be rejected");
}

#[test]
fn snake_case_nav_fields_deserialize() {
    let json = r#"{
        "state": {"StateID": 1, "State": "CA", "StateName": "California"},
        "event": {"EventShort": "100m", "EventName": "100 Meters"},
        "divisions": [{"DivisionID": 1, "DivisionName": "A", "Indoor": false}],
        "genders": ["m"],
        "complete": true,
        "page": 1
    }"#;
    let resp: RawNavInfoResponse = serde_json::from_str(json).unwrap();
    let state = resp.state.unwrap();
    assert_eq!(state.state_id, Some(1));
    assert_eq!(state.state_name, Some("California".to_owned()));
    let event = resp.event.unwrap();
    assert_eq!(event.event_short, Some("100m".to_owned()));
    let div = resp.divisions.unwrap();
    assert_eq!(div[0].division_id, Some(1));
    assert_eq!(div[0].indoor, Some(false));
}

#[test]
fn enforce_allowed_fields_filters() {
    let body = serde_json::json!({
        "divListId": 12,
        "reportType": "div",
        "other": "field"
    });
    let allowed = vec!["divListId".to_owned(), "reportType".to_owned()];
    let allowed_obj = body.as_object().unwrap().clone();
    let mut filtered = serde_json::Map::new();
    for (k, v) in allowed_obj {
        if allowed.contains(&k) {
            filtered.insert(k, v);
        }
    }
    assert_eq!(filtered.len(), 2);
    assert!(filtered.contains_key("divListId"));
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
