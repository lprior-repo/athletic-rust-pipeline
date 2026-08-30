#[cfg(test)]
mod negative_season_regression_tests {
    use crate::alpha_model_raw::RawRankingsResponse;

    // Regression: nested Results with negative SeasonID must be rejected
    #[test]
    fn nested_result_negative_season_id_rejected() {
        let json = r#"{
            "groupedRankings": [[{
                "AthleteID": 1,
                "AthleteName": "Test",
                "GradeID": 2,
                "TeamName": "School",
                "State": "CA",
                "Results": [{
                    "MeetID": 100,
                    "MeetName": "Finals",
                    "IDResult": 500,
                    "EventShort": "100m",
                    "Measure": "10.55",
                    "ResultDate": "2026-06-15",
                    "SeasonID": -1
                }]
            }]]
        }"#;
        let resp = RawRankingsResponse::from_json(json).expect("parse should succeed");
        let result = resp.grouped_rankings[0][0].to_flattened_records();
        assert!(
            result.is_err(),
            "negative SeasonID in nested Results must be rejected, got: {:?}",
            result
        );
    }

    // Regression: flattened row with negative SeasonID must be rejected
    #[test]
    fn flattened_negative_season_id_rejected() {
        let json = r#"{
            "groupedRankings": [[{
                "AthleteID": 1,
                "AthleteName": "Test",
                "GradeID": 2,
                "TeamName": "School",
                "State": "CA",
                "IDResult": 500,
                "EventShort": "100m",
                "Measure": "10.55",
                "ResultDate": "2026-06-15",
                "SeasonID": -1,
                "MeetID": 100,
                "MeetName": "Finals"
            }]]
        }"#;
        let resp = RawRankingsResponse::from_json(json).expect("parse should succeed");
        let result = resp.grouped_rankings[0][0].to_flattened_records();
        assert!(
            result.is_err(),
            "negative SeasonID in flattened row must be rejected, got: {:?}",
            result
        );
    }

    // Regression: nested Results with SeasonID == 0 still rejected
    #[test]
    fn nested_result_zero_season_id_rejected() {
        let json = r#"{
            "groupedRankings": [[{
                "AthleteID": 1,
                "AthleteName": "Test",
                "GradeID": 2,
                "TeamName": "School",
                "State": "CA",
                "Results": [{
                    "MeetID": 100,
                    "MeetName": "Finals",
                    "IDResult": 500,
                    "EventShort": "100m",
                    "Measure": "10.55",
                    "ResultDate": "2026-06-15",
                    "SeasonID": 0
                }]
            }]]
        }"#;
        let resp = RawRankingsResponse::from_json(json).expect("parse should succeed");
        let result = resp.grouped_rankings[0][0].to_flattened_records();
        assert!(result.is_err(), "zero SeasonID in nested Results must be rejected");
    }

    // Regression: flattened row with SeasonID == 0 still rejected
    #[test]
    fn flattened_zero_season_id_rejected() {
        let json = r#"{
            "groupedRankings": [[{
                "AthleteID": 1,
                "AthleteName": "Test",
                "GradeID": 2,
                "TeamName": "School",
                "State": "CA",
                "IDResult": 500,
                "EventShort": "100m",
                "Measure": "10.55",
                "ResultDate": "2026-06-15",
                "SeasonID": 0,
                "MeetID": 100,
                "MeetName": "Finals"
            }]]
        }"#;
        let resp = RawRankingsResponse::from_json(json).expect("parse should succeed");
        let result = resp.grouped_rankings[0][0].to_flattened_records();
        assert!(result.is_err(), "zero SeasonID in flattened row must be rejected");
    }

    // Positive regression: valid positive SeasonID should still work
    #[test]
    fn positive_season_id_accepted() {
        let json = r#"{
            "groupedRankings": [[{
                "AthleteID": 1,
                "AthleteName": "Test",
                "GradeID": 2,
                "TeamName": "School",
                "State": "CA",
                "Results": [{
                    "MeetID": 100,
                    "MeetName": "Finals",
                    "IDResult": 500,
                    "EventShort": "100m",
                    "Measure": "10.55",
                    "ResultDate": "2026-06-15",
                    "SeasonID": 2026
                }]
            }]]
        }"#;
        let resp = RawRankingsResponse::from_json(json).expect("parse should succeed");
        let result = resp.grouped_rankings[0][0].to_flattened_records();
        assert!(result.is_ok(), "positive SeasonID must be accepted");
        assert_eq!(result.unwrap()[0].season_id, 2026);
    }
}

