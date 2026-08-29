use crate::alpha_model_raw::RawNavInfoResponse;

fn parse_json(text: &str) -> RawNavInfoResponse {
    serde_json::from_str(text).expect("valid JSON for test")
}

#[test]
fn empty_object_rejected() {
    let resp = parse_json("{}");
    assert!(resp.validate().is_err());
}

#[test]
fn complete_only_rejected() {
    let resp = parse_json(r#"{"complete":true}"#);
    assert!(resp.validate().is_err());
}

#[test]
fn state_missing_rejected() {
    let json = r#"{
        "state": null,
        "event": {"EventShort":"100m","EventName":"100 Meters"},
        "divisions": [{"DivisionID":1,"DivisionName":"Div","Indoor":false}],
        "genders": ["m"],
        "complete": true,
        "page": 1
    }"#;
    let resp = parse_json(json);
    assert!(resp.state.is_none());
    assert!(resp.validate().is_err());
}

#[test]
fn state_missing_state_id_rejected() {
    let json = r#"{
        "state": {"State":"TS","StateName":"Test"},
        "event": {"EventShort":"100m","EventName":"100 Meters"},
        "divisions": [{"DivisionID":1,"DivisionName":"Div","Indoor":false}],
        "genders": ["m"],
        "complete": true,
        "page": 1
    }"#;
    let resp = parse_json(json);
    assert!(resp.validate().is_err());
}

#[test]
fn state_zero_state_id_rejected() {
    let json = r#"{
        "state": {"StateID":0,"State":"TS","StateName":"Test"},
        "event": {"EventShort":"100m","EventName":"100 Meters"},
        "divisions": [{"DivisionID":1,"DivisionName":"Div","Indoor":false}],
        "genders": ["m"],
        "complete": true,
        "page": 1
    }"#;
    let resp = parse_json(json);
    assert!(resp.validate().is_err());
}

#[test]
fn state_empty_state_name_rejected() {
    let json = r#"{
        "state": {"StateID":1,"State":"TS","StateName":""},
        "event": {"EventShort":"100m","EventName":"100 Meters"},
        "divisions": [{"DivisionID":1,"DivisionName":"Div","Indoor":false}],
        "genders": ["m"],
        "complete": true,
        "page": 1
    }"#;
    let resp = parse_json(json);
    assert!(resp.validate().is_err());
}

#[test]
fn event_missing_rejected() {
    let json = r#"{
        "state": {"StateID":1,"State":"TS","StateName":"Test"},
        "divisions": [{"DivisionID":1,"DivisionName":"Div","Indoor":false}],
        "genders": ["m"],
        "complete": true,
        "page": 1
    }"#;
    let resp = parse_json(json);
    assert!(resp.event.is_none());
    assert!(resp.validate().is_err());
}

#[test]
fn event_empty_event_short_rejected() {
    let json = r#"{
        "state": {"StateID":1,"State":"TS","StateName":"Test"},
        "event": {"EventShort":"","EventName":"100 Meters"},
        "divisions": [{"DivisionID":1,"DivisionName":"Div","Indoor":false}],
        "genders": ["m"],
        "complete": true,
        "page": 1
    }"#;
    let resp = parse_json(json);
    assert!(resp.validate().is_err());
}

#[test]
fn divisions_empty_rejected() {
    let json = r#"{
        "state": {"StateID":1,"State":"TS","StateName":"Test"},
        "event": {"EventShort":"100m","EventName":"100 Meters"},
        "divisions": [],
        "genders": ["m"],
        "complete": true,
        "page": 1
    }"#;
    let resp = parse_json(json);
    assert!(resp.validate().is_err());
}

#[test]
fn divisions_missing_rejected() {
    let json = r#"{
        "state": {"StateID":1,"State":"TS","StateName":"Test"},
        "event": {"EventShort":"100m","EventName":"100 Meters"},
        "genders": ["m"],
        "complete": true,
        "page": 1
    }"#;
    let resp = parse_json(json);
    assert!(resp.divisions.is_none());
    assert!(resp.validate().is_err());
}

#[test]
fn division_missing_name_rejected() {
    let json = r#"{
        "state": {"StateID":1,"State":"TS","StateName":"Test"},
        "event": {"EventShort":"100m","EventName":"100 Meters"},
        "divisions": [{"DivisionID":1,"Indoor":false}],
        "genders": ["m"],
        "complete": true,
        "page": 1
    }"#;
    let resp = parse_json(json);
    assert!(resp.validate().is_err());
}

#[test]
fn division_zero_id_rejected() {
    let json = r#"{
        "state": {"StateID":1,"State":"TS","StateName":"Test"},
        "event": {"EventShort":"100m","EventName":"100 Meters"},
        "divisions": [{"DivisionID":0,"DivisionName":"Div","Indoor":false}],
        "genders": ["m"],
        "complete": true,
        "page": 1
    }"#;
    let resp = parse_json(json);
    assert!(resp.validate().is_err());
}

#[test]
fn division_missing_indoor_rejected() {
    let json = r#"{
        "state": {"StateID":1,"State":"TS","StateName":"Test"},
        "event": {"EventShort":"100m","EventName":"100 Meters"},
        "divisions": [{"DivisionID":1,"DivisionName":"Div"}],
        "genders": ["m"],
        "complete": true,
        "page": 1
    }"#;
    let resp = parse_json(json);
    assert!(resp.validate().is_err());
}

#[test]
fn genders_empty_rejected() {
    let json = r#"{
        "state": {"StateID":1,"State":"TS","StateName":"Test"},
        "event": {"EventShort":"100m","EventName":"100 Meters"},
        "divisions": [{"DivisionID":1,"DivisionName":"Div","Indoor":false}],
        "genders": [],
        "complete": true,
        "page": 1
    }"#;
    let resp = parse_json(json);
    assert!(resp.validate().is_err());
}

#[test]
fn genders_missing_rejected() {
    let json = r#"{
        "state": {"StateID":1,"State":"TS","StateName":"Test"},
        "event": {"EventShort":"100m","EventName":"100 Meters"},
        "divisions": [{"DivisionID":1,"DivisionName":"Div","Indoor":false}],
        "complete": true,
        "page": 1
    }"#;
    let resp = parse_json(json);
    assert!(resp.genders.is_none());
    assert!(resp.validate().is_err());
}

#[test]
fn no_pagination_fields_rejected() {
    let json = r#"{
        "state": {"StateID":1,"State":"TS","StateName":"Test"},
        "event": {"EventShort":"100m","EventName":"100 Meters"},
        "divisions": [{"DivisionID":1,"DivisionName":"Div","Indoor":false}],
        "genders": ["m"]
    }"#;
    let resp = parse_json(json);
    assert!(resp.validate().is_err());
}

#[test]
fn only_page_pagination_accepted() {
    let json = r#"{
        "state": {"StateID":1,"State":"TS","StateName":"Test"},
        "event": {"EventShort":"100m","EventName":"100 Meters"},
        "divisions": [{"DivisionID":1,"DivisionName":"Div","Indoor":false}],
        "genders": ["m"],
        "page": 1
    }"#;
    let resp = parse_json(json);
    assert!(resp.validate().is_ok());
}

#[test]
fn fixture_redacted_accepted() {
    let text = std::fs::read_to_string("fixtures/alpha/get-nav-info-redacted.json")
        .expect("fixture must exist");
    let resp: RawNavInfoResponse = serde_json::from_str(&text)
        .expect("fixture must parse");
    assert!(resp.validate().is_ok());
}
