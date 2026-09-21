use super::*;
use crate::domain::evidence::Sport;
use crate::domain::identity::AthleteId;

#[test]
fn search_body_is_exact_and_encoded() {
    let origin = Url::parse("http://127.0.0.1:8080/").expect("origin");
    let request = build(
        &origin,
        &SourceResource::Search {
            query: "Ada Example".to_owned(),
            sport: Sport::TrackField,
            start: 12,
        },
    )
    .expect("request");
    assert_eq!(request.url.path(), "/Search.aspx/runSearch");
    assert_eq!(
        serde_json::to_value(request.body()).expect("json"),
        serde_json::json!({"q":"Ada Example","fq":"t:a a:tf","start":12})
    );
}

#[test]
fn captured_fetch_requests_survive_durable_serialization() -> anyhow::Result<()> {
    let origin = Url::parse("http://127.0.0.1:8080/")?;
    let resources = [
        SourceResource::Search {
            query: "Ada Example".to_owned(),
            sport: Sport::TrackField,
            start: 12,
        },
        SourceResource::Bio {
            athlete_id: AthleteId::new(7)?,
            sport: Sport::CrossCountry,
        },
    ];
    for resource in resources {
        let request = build(&origin, &resource)?;
        let bytes = serde_json::to_vec(&request)?;
        let restored: RequestSpec = serde_json::from_slice(&bytes)?;
        assert_eq!(restored, request);
    }
    Ok(())
}

#[test]
fn unsafe_search_and_profile_inputs_are_rejected() {
    let origin = Url::parse("http://127.0.0.1:8080/").expect("origin");
    assert!(build(
        &origin,
        &SourceResource::Search {
            query: "x\ninvalid".to_owned(),
            sport: Sport::CrossCountry,
            start: 0
        }
    )
    .is_err());
    let id = AthleteId::new(7).expect("id");
    let profile = ProfileUrl::parse("https://athletic.net/athlete/7").expect("profile");
    assert_eq!(profile.athlete_id(), id);
    assert!(build(
        &origin,
        &SourceResource::ProfileHtml {
            profile_url: profile
        }
    )
    .is_ok());
}
// -- Rankings body encoding and wire layout --

#[test]
fn rankings_query_encodes_exact_measured_body() {
    let origin = Url::parse("http://127.0.0.1:8080/").expect("origin");
    let request = rankings_spec(
        &origin,
        RankingsAction {
            list_id: 168416,
            gender: "m".into(),
            grade: Some(11),
            event_short: "100m".into(),
            page: 1,
            capture: RankingsCapture::Navigation,
        },
    )
    .expect("request");
    // Verify body serialises to the byte-verified measured request body.
    let body = request.body().expect("has body");
    match body {
        RequestBody::Rankings(_) => {}
        _ => panic!("expected Rankings body"),
    }
    let json = serde_json::to_string(&body).expect("json");
    let expected = r#"{"reportType":"div","mode":"list","divListId":168416,"indoor":null,"eventShort":"100m","gender":"m","qParams":{"grades":[11],"page":1},"qualifyingListKey":"","version":2,"debug":""}"#;
    assert_eq!(json, expected);
}

#[test]
fn rankings_grade_none_emits_empty_array_not_null() {
    let origin = Url::parse("http://127.0.0.1:8080/").expect("origin");
    let request = rankings_spec(
        &origin,
        RankingsAction {
            list_id: 168416,
            gender: "f".into(),
            grade: None,
            event_short: "200m".into(),
            page: 2,
            capture: RankingsCapture::Results,
        },
    )
    .expect("request");
    let json = serde_json::to_string(&request.body().expect("body")).expect("json");
    assert!(json.contains(r#""grades":[]"#));
    assert!(!json.contains(r#""grades":null"#));
}

#[test]
fn rankings_semantic_url_matches_legacy_ui_url() {
    let origin = Url::parse("http://127.0.0.1:8080/").expect("origin");
    // With grade
    let req = rankings_spec(
        &origin,
        RankingsAction {
            list_id: 168416,
            gender: "m".into(),
            grade: Some(11),
            event_short: "100m".into(),
            page: 3,
            capture: RankingsCapture::Navigation,
        },
    )
    .expect("request");
    assert_eq!(
        req.semantic_url,
        "http://127.0.0.1:8080/TrackAndField/rankings/list/168416/m/100m/?page=3&grades=11"
    );
    // Without grade — use a valid event_short (empty fails safe_text validation)
    let req2 = rankings_spec(
        &origin,
        RankingsAction {
            list_id: 99,
            gender: "f".into(),
            grade: None,
            event_short: "TF".into(),
            page: 1,
            capture: RankingsCapture::Results,
        },
    )
    .expect("request");
    assert_eq!(
        req2.semantic_url,
        "http://127.0.0.1:8080/TrackAndField/rankings/list/99/f/TF/?page=1"
    );
}

#[test]
fn rankings_physical_url_is_api_endpoint_empty_query() {
    let origin = Url::parse("http://127.0.0.1:8080/").expect("origin");
    let request = rankings_spec(
        &origin,
        RankingsAction {
            list_id: 168416,
            gender: "m".into(),
            grade: Some(11),
            event_short: "100m".into(),
            page: 1,
            capture: RankingsCapture::Navigation,
        },
    )
    .expect("request");
    assert_eq!(
        request.url.as_str(),
        "http://127.0.0.1:8080/api/v1/tfRankings/GetRankings"
    );
    assert!(request.url.query().is_none());
}

#[test]
fn rankings_rejects_zero_page_and_oversized_event() {
    let origin = Url::parse("http://127.0.0.1:8080/").expect("origin");
    assert!(rankings_spec(
        &origin,
        RankingsAction {
            list_id: 1,
            gender: "m".into(),
            grade: None,
            event_short: "100m".into(),
            page: 0,
            capture: RankingsCapture::Navigation,
        },
    )
    .is_err());
    // Event text exceeding 64 bytes
    let long_event = "x".repeat(65);
    assert!(rankings_spec(
        &origin,
        RankingsAction {
            list_id: 1,
            gender: "m".into(),
            grade: None,
            event_short: long_event,
            page: 1,
            capture: RankingsCapture::Navigation,
        },
    )
    .is_err());
}
