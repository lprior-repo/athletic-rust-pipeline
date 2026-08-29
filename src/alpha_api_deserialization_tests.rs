use crate::alpha_model_raw::{RawNavInfoResponse, RawRankingRecord, RawRankingResult, RawRankingsResponse};

// --- Fixture deserialization ---
#[test]
fn deserialize_redacted_rankings_fixture() {
    let fixture = std::fs::read_to_string("fixtures/alpha/get-rankings-redacted.json").unwrap();
    let resp = RawRankingsResponse::from_json(&fixture).unwrap();
    assert_eq!(resp.grouped_rankings.len(), 2);
    assert_eq!(resp.grouped_rankings[0].len(), 2);
    let rec = &resp.grouped_rankings[0][0];
    assert_eq!(rec.athlete_id, 90_000_001);
    assert_eq!(rec.results.as_ref().unwrap()[0].meet_id, 90_000_001);
    assert_eq!(rec.results.as_ref().unwrap()[0].meet_name, "Test Meet");
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
    assert!(rec.results.is_none() || rec.results.as_ref().unwrap().is_empty());
}
#[test]
fn to_flattened_records_errors_when_no_valid_data() {
    let json = r#"{
        "AthleteID": 999,
        "AthleteName": "Nobody",
        "GradeID": 0,
        "TeamName": "",
        "State": "",
        "Results": []
    }"#;
    let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
    let result = rec.to_flattened_records();
    assert!(result.is_err(), "no valid data should error, not return empty");
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
