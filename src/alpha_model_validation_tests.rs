#[cfg(test)]
mod raw_ranking_record_tests {
    use crate::alpha_model_raw::RawRankingRecord;

    #[test]
    fn required_fields_must_be_present_in_raw_ranking_record() {
        let json = r#"{
            "AthleteID": 123
        }"#;
        let result: Result<RawRankingRecord, _> = serde_json::from_str(json);
        assert!(result.is_err(), "missing required fields must cause deserialization error");
    }

    #[test]
    fn unknown_fields_in_raw_ranking_record_ignored() {
        let json = r#"{
            "AthleteID": 1,
            "AthleteName": "Test",
            "GradeID": 2,
            "TeamName": "School",
            "State": "CA",
            "UnknownField": "ignored",
            "AnotherUnknown": 42
        }"#;
        let rec: RawRankingRecord = serde_json::from_str(json)
            .expect("unknown fields should be ignored");
        assert_eq!(rec.athlete_id, 1);
        assert_eq!(rec.athlete_name, "Test");
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
        assert!(records.is_err(), "missing required fields should error, not empty");
    }

    #[test]
    fn from_flattened_rejects_zero_id_result() {
        let json = r#"{
            "AthleteID": 1, "AthleteName": "Test", "GradeID": 2,
            "TeamName": "School", "State": "CA",
            "IDResult": 0, "EventShort": "100m", "Measure": "10.5s",
            "ResultDate": "2024-01-01", "SeasonID": 1, "MeetID": 5, "MeetName": "Meet"
        }"#;
        let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
        let result = rec.to_flattened_records();
        assert!(result.is_err(), "zero IDResult must error");
    }

    #[test]
    fn from_flattened_rejects_empty_event_short() {
        let json = r#"{
            "AthleteID": 1, "AthleteName": "Test", "GradeID": 2,
            "TeamName": "School", "State": "CA",
            "IDResult": 100, "EventShort": "", "Measure": "10.5s",
            "ResultDate": "2024-01-01", "SeasonID": 1, "MeetID": 5, "MeetName": "Meet"
        }"#;
        let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
        let result = rec.to_flattened_records();
        assert!(result.is_err(), "empty EventShort must error");
    }

    #[test]
    fn from_flattened_rejects_zero_season_id() {
        let json = r#"{
            "AthleteID": 1, "AthleteName": "Test", "GradeID": 2,
            "TeamName": "School", "State": "CA",
            "IDResult": 100, "EventShort": "100m", "Measure": "10.5s",
            "ResultDate": "2024-01-01", "SeasonID": 0, "MeetID": 5, "MeetName": "Meet"
        }"#;
        let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
        let result = rec.to_flattened_records();
        assert!(result.is_err(), "zero SeasonID must error");
    }

    #[test]
    fn from_flattened_rejects_zero_meet_id() {
        let json = r#"{
            "AthleteID": 1, "AthleteName": "Test", "GradeID": 2,
            "TeamName": "School", "State": "CA",
            "IDResult": 100, "EventShort": "100m", "Measure": "10.5s",
            "ResultDate": "2024-01-01", "SeasonID": 1, "MeetID": 0, "MeetName": "Meet"
        }"#;
        let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
        let result = rec.to_flattened_records();
        assert!(result.is_err(), "zero MeetID must error");
    }

    #[test]
    fn from_flattened_rejects_empty_meet_name() {
        let json = r#"{
            "AthleteID": 1, "AthleteName": "Test", "GradeID": 2,
            "TeamName": "School", "State": "CA",
            "IDResult": 100, "EventShort": "100m", "Measure": "10.5s",
            "ResultDate": "2024-01-01", "SeasonID": 1, "MeetID": 5, "MeetName": ""
        }"#;
        let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
        let result = rec.to_flattened_records();
        assert!(result.is_err(), "empty MeetName must error");
    }

    #[test]
    fn from_flattened_rejects_empty_measure() {
        let json = r#"{
            "AthleteID": 1, "AthleteName": "Test", "GradeID": 2,
            "TeamName": "School", "State": "CA",
            "IDResult": 100, "EventShort": "100m", "Measure": "",
            "ResultDate": "2024-01-01", "SeasonID": 1, "MeetID": 5, "MeetName": "Meet"
        }"#;
        let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
        let result = rec.to_flattened_records();
        assert!(result.is_err(), "empty Measure must error");
    }

    #[test]
    fn from_flattened_rejects_empty_result_date() {
        let json = r#"{
            "AthleteID": 1, "AthleteName": "Test", "GradeID": 2,
            "TeamName": "School", "State": "CA",
            "IDResult": 100, "EventShort": "100m", "Measure": "10.5s",
            "ResultDate": "", "SeasonID": 1, "MeetID": 5, "MeetName": "Meet"
        }"#;
        let rec: RawRankingRecord = serde_json::from_str(json).unwrap();
        let result = rec.to_flattened_records();
        assert!(result.is_err(), "empty ResultDate must error");
    }
}

#[cfg(test)]
mod raw_ranking_result_tests {
    use crate::alpha_model_raw::RawRankingResult;

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
}

#[cfg(test)]
mod raw_rankings_response_tests {
    use crate::alpha_model_raw::RawRankingsResponse;

    #[test]
    fn required_fields_must_be_present_in_raw_rankings_response() {
        let json = r#"{}"#;
        let result: Result<RawRankingsResponse, _> = serde_json::from_str(json);
        assert!(result.is_err(), "missing groupedRankings must cause deserialization error");
    }

    #[test]
    fn from_json_requires_grouped_rankings() {
        let result = RawRankingsResponse::from_json(r#"{}"#);
        assert!(result.is_err(), "missing groupedRankings must error");
    }

    #[test]
    fn from_json_rejects_non_array_grouped_rankings() {
        let result = RawRankingsResponse::from_json(r#"{"groupedRankings": "bad"}"#);
        assert!(result.is_err(), "non-array groupedRankings must error");
    }

    #[test]
    fn from_json_rejects_non_array_group() {
        let result = RawRankingsResponse::from_json(
            r#"{"groupedRankings": [{"AthleteID": 1}]}"#
        );
        assert!(result.is_err(), "non-array group must error");
    }

    #[test]
    fn from_json_rejects_malformed_row() {
        // Missing required AthleteName field
        let result = RawRankingsResponse::from_json(
            r#"{"groupedRankings": [[{"AthleteID": 1}]]}"#
        );
        assert!(result.is_err(), "malformed row must error");
    }

    #[test]
    fn from_json_propagates_continuation_error() {
        let result = RawRankingsResponse::from_json(
            r#"{"groupedRankings": [], "continuation": "bad"}"#
        );
        assert!(result.is_err(), "malformed continuation must error");
    }

    #[test]
    fn from_json_ignores_unknown_response_fields() {
        let json = r#"{"groupedRankings":[],"page":1,"complete":true,"unknown_field":"ignored"}"#;
        let raw = RawRankingsResponse::from_json(json).expect("unknown fields should be ignored");
        assert_eq!(raw.page, Some(1));
    }

    #[test]
    fn from_json_rejects_wrong_page_type() {
        // page must be u64 or null; string type should error
        let result = RawRankingsResponse::from_json(
            r#"{"groupedRankings":[],"page":"invalid"}"#,
        );
        assert!(result.is_err(), "string page must error");
    }

    #[test]
    fn from_json_rejects_float_page_type() {
        let result = RawRankingsResponse::from_json(
            r#"{"groupedRankings":[],"page":1.5}"#,
        );
        assert!(result.is_err(), "float page must error");
    }

    #[test]
    fn from_json_accepts_null_page() {
        let raw = RawRankingsResponse::from_json(
            r#"{"groupedRankings":[],"page":null}"#,
        ).expect("null page should be accepted");
        assert_eq!(raw.page, None);
    }

    #[test]
    fn from_json_preserves_value() {
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
