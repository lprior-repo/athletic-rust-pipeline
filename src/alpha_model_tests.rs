#[cfg(test)]
mod tests {
    use crate::alpha_model::*;
    use std::path::Path;
    #[test]
    fn parse_rankings_fixture() {
        let fixture = std::fs::read_to_string(
            Path::new("fixtures/alpha/get-rankings-redacted.json"),
        )
        .expect("fixture must exist");
        let resp: RawRankingsResponse =
            serde_json::from_str(&fixture).expect("fixture must deserialize");

        // groupedRankings is present with correct nesting.
        assert_eq!(resp.grouped_rankings.len(), 2);
        assert_eq!(resp.grouped_rankings[0].len(), 2); // first group has 2 records.
        assert_eq!(resp.grouped_rankings[1].len(), 0); // second group is empty.

        // First record fields match fixture PascalCase keys.
        let rec = &resp.grouped_rankings[0][0];
        assert_eq!(rec.athlete_id, 90_000_001);
        assert_eq!(rec.athlete_name, "Test Runner");
        assert_eq!(rec.state, "TS");
        assert_eq!(rec.results.len(), 1);

        let r0 = &rec.results[0];
        assert_eq!(r0.measure, "10.50");
        assert_eq!(r0.wind, Some("+1.2".to_owned()));
        assert_eq!(r0.event_short, "100m");

        // Second record (same athlete, different event).
        let rec2 = &resp.grouped_rankings[0][1];
        assert_eq!(rec2.athlete_id, 90_000_001);
        assert_eq!(rec2.athlete_name, "Test Runner");
        assert_eq!(rec2.state, "TS");
        assert_eq!(rec2.results.len(), 1);
        assert_eq!(rec2.results[0].measure, "21.30");
        assert_eq!(rec2.results[0].event_short, "200m");
        assert_eq!(rec2.results[0].wind, None);

        // Continuation metadata present.
        assert_eq!(resp.continuation.as_ref().unwrap().page, 2);
        assert!(!resp.continuation.as_ref().unwrap().complete);
    }

    #[test]
    fn parse_nav_info_fixture() {
        let fixture = std::fs::read_to_string(
            Path::new("fixtures/alpha/get-nav-info-redacted.json"),
        )
        .expect("fixture must exist");
        let resp: RawNavInfoResponse =
            serde_json::from_str(&fixture).expect("fixture must deserialize");
        let state = resp.state.unwrap();
        assert_eq!(state.StateID, Some(90_000_001));
        assert_eq!(state.State, Some("TS".to_owned()));
        assert_eq!(state.StateName, Some("Test State".to_owned()));

        let event = resp.event.unwrap();
        assert_eq!(event.EventShort, Some("100m".to_owned()));
        assert_eq!(event.EventName, Some("100 Meters".to_owned()));

        let div = resp.divisions.unwrap();
        assert_eq!(div.len(), 1);
        assert_eq!(div[0].DivisionID, Some(90_000_001));
        assert_eq!(div[0].DivisionName, Some("Test Division".to_owned()));
        assert_eq!(div[0].Indoor, Some(false));

        assert_eq!(resp.genders, Some(vec!["m".to_owned()]));
        assert_eq!(resp.complete, Some(true));
        assert_eq!(resp.page, Some(1));
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
            "UnknownField": "ignored",
            "AnotherUnknown": 42
        }"#;
        let result: RawRankingResult = serde_json::from_str(json)
            .expect("unknown fields should be ignored");
        assert_eq!(result.meet_id, 123);
        assert_eq!(result.measure, "10.5");
    }

    #[test]
    fn required_fields_must_be_present_in_raw_ranking_result() {
        let json = r#"{
            "MeetID": 123
        }"#;
        let result: Result<RawRankingResult, _> = serde_json::from_str(json);
        assert!(result.is_err(), "missing required fields must cause deserialization error");
    }

    #[test]
    fn required_fields_must_be_present_in_raw_ranking_record() {
        let json = r#"{
            "AthleteID": 123
        }"#;
        let result: Result<RawRankingRecord, _> = serde_json::from_str(json);
        assert!(result.is_err(), "missing required fields must cause deserialization error");
    }

    #[test]
    fn required_fields_must_be_present_in_raw_rankings_response() {
        let json = r#"{}"#;
        let result: Result<RawRankingsResponse, _> = serde_json::from_str(json);
        assert!(result.is_err(), "missing groupedRankings must cause deserialization error");
    }
    #[test]
    fn ranking_record_serializes_with_meet_fields() {
        let rec = RankingRecord {
            athlete_id: 1,
            athlete_name: "Test".into(),
            grade_id: 2,
            team_name: "School".into(),
            state: "CA".into(),
            meet_id: 100,
            meet_name: "Meet A".into(),
            result_id: Some(500),
            event_short: "100m".into(),
            measure: "10.5".into(),
            result_date: "2026-01-01".into(),
            season_id: 2026,
            wind: None,
        };
        let json = serde_json::to_string(&rec).unwrap();
        assert!(json.contains("\"meet_id\""));
        assert!(json.contains("\"meet_name\""));
    }

    #[test]
    fn raw_ranking_record_deserializes_with_meet_fields() {
        let json = r#"{
            "AthleteID": 1,
            "AthleteName": "Test",
            "GradeID": 2,
            "TeamName": "School",
            "State": "CA",
            "MeetID": 100,
            "MeetName": "Meet A",
            "Results": []
        }"#;
        let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
        assert_eq!(rec.meet_id, Some(100));
        assert_eq!(rec.meet_name, Some("Meet A".into()));
    }

    #[test]
    fn to_flattened_records_preserves_meet_fields() {
        let json = r#"{
            "AthleteID": 1,
            "AthleteName": "Test",
            "GradeID": 2,
            "TeamName": "School",
            "State": "CA",
            "Results": [{
                "MeetID": 100,
                "MeetName": "Meet A",
                "IDResult": 500,
                "EventShort": "100m",
                "Measure": "10.5",
                "ResultDate": "2026-01-01",
                "SeasonID": 2026,
                "Wind": null
            }]
        }"#;
        let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
        let records = rec.to_flattened_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].meet_id, 100);
        assert_eq!(records[0].meet_name, "Meet A");
    }

    #[test]
    fn to_flattened_records_flattened_row_with_meet() {
        let json = r#"{
            "AthleteID": 1,
            "AthleteName": "Test",
            "GradeID": 2,
            "TeamName": "School",
            "State": "CA",
            "IDResult": 500,
            "EventShort": "100m",
            "Measure": "10.5",
            "ResultDate": "2026-01-01",
            "SeasonID": 2026,
            "MeetID": 100,
            "MeetName": "Meet A"
        }"#;
        let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
        let records = rec.to_flattened_records();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].meet_id, 100);
        assert_eq!(records[0].meet_name, "Meet A");
    }

    #[test]
    fn to_flattened_records_rejects_missing_required() {
        let json = r#"{
            "AthleteID": 1,
            "AthleteName": "Test",
            "GradeID": 2,
            "TeamName": "School",
            "State": "CA",
            "EventShort": "100m"
        }"#;
        let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
        let records = rec.to_flattened_records();
        assert!(records.is_empty(), "missing required fields should produce empty");
    }

    #[test]
    fn raw_rankings_response_from_json_preserves_value() {
        let json = r#"{
            "groupedRankings": [[{
                "AthleteID": 1,
                "AthleteName": "Test",
                "GradeID": 2,
                "TeamName": "School",
                "State": "CA",
                "Results": []
            }]],
            "complete": true,
            "unknownField": "preserved"
        }"#;
        let resp = RawRankingsResponse::from_json(json).unwrap();
        assert_eq!(resp.grouped_rankings.len(), 1);
        assert_eq!(resp.complete, Some(serde_json::json!(true)));
        assert!(resp.value.get("unknownField").is_some());
        assert_eq!(resp.value.pointer("/complete"), Some(&serde_json::json!(true)));
    }
}
