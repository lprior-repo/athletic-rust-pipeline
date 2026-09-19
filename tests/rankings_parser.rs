use athletic_rust_pipeline::runtime::rankings::{
    parse_page_response, ExpectedPageContext, PageParseError,
};
use serde_json::{json, Value};

fn expected() -> ExpectedPageContext<'static> {
    ExpectedPageContext {
        division_id: 168_416,
        season_id: 2026,
        gender: "m",
        event_short: "4x100",
        event_id: Some(168_416),
        is_relay: true,
        requested_grade: None,
        page: 1,
    }
}

fn payload(row_athlete_id: Value, relay_teams: Option<Value>) -> Value {
    let mut response = json!({
        "division": {
            "ID": 168416,
            "SeasonID": 2026,
            "BaseDiv": {"Country": "USA", "Level": 4}
        },
        "gender": "m",
        "eventId": 168416,
        "eventShort": "4x100",
        "settings": {"page": 1, "depth": 100, "grades": []},
        "minCount": 1,
        "groupedRankings": [[{
            "rowNum": 1,
            "IDResult": 9001,
            "AthleteID": row_athlete_id,
            "GradeID": 99
        }]]
    });
    if let Some(teams) = relay_teams {
        response["relayTeams"] = teams;
    }
    response
}

#[test]
fn absent_roster_keeps_source_row_and_counts_missing_without_candidate() {
    let raw = payload(json!(-7), None);
    let observation = parse_page_response(&raw, &expected()).expect("synthetic payload is valid");

    assert_eq!(observation.source_rows.len(), 1);
    assert_eq!(observation.source_rows[0].roster_present, Some(false));
    assert_eq!(observation.rows_missing_roster, 1);
    assert_eq!(observation.rows_with_roster, 0);
    assert!(observation.verified_relay_members.is_empty());
}

#[test]
fn unknown_roster_result_is_missing_not_a_fake_zero_join() {
    let raw = payload(
        json!(77),
        Some(json!({"9002": {
            "IDResult": 9002,
            "RelayTeamID": 77,
            "Members": []
        }})),
    );
    let observation = parse_page_response(&raw, &expected()).expect("synthetic payload is valid");

    assert_eq!(observation.source_rows[0].roster_present, Some(false));
    assert_eq!(observation.rows_missing_roster, 1);
    assert!(observation.verified_relay_members.is_empty());
}

#[test]
fn existing_roster_rejects_negative_row_team_id() {
    let raw = payload(
        json!(-7),
        Some(json!({"9001": {
            "IDResult": 9001,
            "RelayTeamID": 77,
            "Members": []
        }})),
    );
    let result = parse_page_response(&raw, &expected());

    assert!(matches!(result, Err(PageParseError::WrongRelayTeamId)));
}
