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

fn individual_expected() -> ExpectedPageContext<'static> {
    ExpectedPageContext {
        division_id: 173_005,
        season_id: 12_026,
        gender: "m",
        event_short: "55m",
        event_id: Some(420),
        is_relay: false,
        requested_grade: Some(11),
        page: 1,
    }
}

fn individual_payload(athlete_id: u64, grade_id: u64) -> Value {
    json!({
        "division": {
            "ID": 173005,
            "SeasonID": 12026,
            "BaseDiv": {"Country": "USA", "Level": 4}
        },
        "gender": "m",
        "eventId": 420,
        "eventShort": "55m",
        "settings": {"page": 1, "depth": 100, "grades": [11]},
        "minCount": 1,
        "groupedRankings": [[{
            "rowNum": 1,
            "IDResult": 9001,
            "AthleteID": athlete_id,
            "AthleteName": "Ada Runner",
            "GradeID": grade_id
        }]]
    })
}

#[test]
fn published_individual_ids_keep_the_observation_contribution() {
    let observation = parse_page_response(&individual_payload(4242, 11), &individual_expected())
        .expect("published athlete id row parses");

    assert_eq!(observation.id_results, vec![9001]);
    assert_eq!(observation.grade_11_candidates, 1);
}

#[test]
fn anonymous_rows_stay_source_rows_without_identity_or_candidate() {
    let observation = parse_page_response(&individual_payload(0, 99), &individual_expected())
        .expect("source anonymous rows parse");

    assert_eq!(observation.row_count, 1);
    assert_eq!(observation.source_rows.len(), 1);
    assert_eq!(observation.source_rows[0].result_id, 9001);
    assert!(observation.id_results.is_empty());
    assert_eq!(observation.grade_11_candidates, 0);
    assert_eq!(observation.unresolved_individual_identities, 0);
}

#[test]
fn anonymous_grade_eleven_rows_count_unresolved_instead_of_failing() {
    let observation = parse_page_response(&individual_payload(0, 11), &individual_expected())
        .expect("anonymous grade 11 rows parse");

    assert_eq!(observation.row_count, 1);
    assert_eq!(observation.grade_11_candidates, 0);
    assert_eq!(observation.unresolved_individual_identities, 1);
    assert!(observation.id_results.is_empty());
}
