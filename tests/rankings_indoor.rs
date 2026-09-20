//! Indoor page contract: the kind-specific `seasons` key selects the indoor
//! list, and every page is then bound to that division, so a page served for
//! the wrong seasonal division fails closed instead of entering the collection.

use athletic_rust_pipeline::runtime::rankings::{
    parse_page_response, EventCatalog, ExpectedPageContext, PageParseError, RequestedFamily,
    SeasonKind,
};
use serde_json::{json, Value};

const INDOOR_LIST: u64 = 173_005;
const OUTDOOR_LIST: u64 = 168_416;
const INDOOR_SEASON: u64 = 12_026;
const OUTDOOR_SEASON: u64 = 2_026;

fn requested() -> Vec<RequestedFamily> {
    vec![RequestedFamily {
        group: "sprints".to_owned(),
        family: "100m".to_owned(),
        short: "100m".to_owned(),
    }]
}

fn nav() -> Value {
    json!({
        "divListId": OUTDOOR_LIST,
        "levelDivId": OUTDOOR_LIST,
        "seasons": {"2026": OUTDOOR_LIST, "12026": INDOOR_LIST},
        "events": [{
            "id": 1,
            "name": "100 Meter Dash",
            "t": "T",
            "m": "M",
            "r": false,
            "h": false,
            "short": "100m",
            "w": true,
            "fmt": null,
            "so": 1,
        }],
    })
}

fn page(division_id: u64, season_id: u64, gender: &str) -> Value {
    json!({
        "division": {
            "ID": division_id,
            "SeasonID": season_id,
            "BaseDiv": {"Country": "USA", "Level": 4},
        },
        "gender": gender,
        "eventId": 420,
        "eventShort": "100m",
        "settings": {"page": 1, "depth": 100, "grades": [11]},
        "minCount": 1,
        "groupedRankings": [[{
            "rowNum": 1,
            "IDResult": 9001,
            "AthleteID": 4242,
            "AthleteName": "Ada Runner",
            "GradeID": 11,
        }]],
    })
}

fn indoor_context<'a>(catalog: &'a EventCatalog, gender: &'a str) -> ExpectedPageContext<'a> {
    ExpectedPageContext {
        division_id: catalog.list_id,
        season_id: catalog.season_id,
        gender,
        event_short: "100m",
        event_id: Some(420),
        is_relay: false,
        requested_grade: Some(11),
        page: 1,
    }
}

#[test]
fn indoor_pages_bind_to_the_indoor_division_not_the_outdoor_list() {
    let catalog = EventCatalog::from_nav(&nav(), &requested(), SeasonKind::Indoor, INDOOR_LIST)
        .expect("indoor nav selects the indoor list");
    assert_eq!(catalog.list_id, INDOOR_LIST);
    // The captured nav keys the indoor 2026 season as "12026"; that source
    // season id is what an indoor division page reports as `SeasonID`.
    assert_eq!(catalog.season_id, INDOOR_SEASON);
    let observation = parse_page_response(
        &page(INDOOR_LIST, INDOOR_SEASON, "f"),
        &indoor_context(&catalog, "f"),
    )
    .expect("indoor page belongs to the indoor division");
    assert_eq!(observation.division_id, Some(INDOOR_LIST));
    assert_eq!(observation.season_id, Some(INDOOR_SEASON));
    assert!(observation.is_valid(catalog.list_id, catalog.season_id, 1));
    assert_eq!(observation.row_count, 1);
    assert_eq!(observation.grade_11_candidates, 1);
}

#[test]
fn an_outdoor_season_page_fails_closed_under_an_indoor_scope() {
    let catalog = EventCatalog::from_nav(&nav(), &requested(), SeasonKind::Indoor, INDOOR_LIST)
        .expect("indoor nav selects the indoor list");
    let error = parse_page_response(
        &page(INDOOR_LIST, OUTDOOR_SEASON, "f"),
        &indoor_context(&catalog, "f"),
    )
    .expect_err("an outdoor-season page cannot enter an indoor collection");
    match error {
        PageParseError::SeasonMismatch { expected, actual } => {
            assert_eq!(expected, INDOOR_SEASON);
            assert_eq!(actual, OUTDOOR_SEASON);
        }
        other => panic!("unexpected page error: {other:?}"),
    }
}

#[test]
fn an_outdoor_page_fails_closed_under_an_indoor_scope() {
    let catalog = EventCatalog::from_nav(&nav(), &requested(), SeasonKind::Indoor, INDOOR_LIST)
        .expect("indoor nav selects the indoor list");
    let error = parse_page_response(
        &page(OUTDOOR_LIST, OUTDOOR_SEASON, "f"),
        &indoor_context(&catalog, "f"),
    )
    .expect_err("an outdoor division page cannot enter an indoor collection");
    match error {
        PageParseError::DivisionMismatch { expected, actual } => {
            assert_eq!(expected, INDOOR_LIST);
            assert_eq!(actual, OUTDOOR_LIST);
        }
        other => panic!("unexpected page error: {other:?}"),
    }
}

#[test]
fn a_boys_page_fails_closed_under_a_girls_scope() {
    let catalog = EventCatalog::from_nav(&nav(), &requested(), SeasonKind::Indoor, INDOOR_LIST)
        .expect("indoor nav selects the indoor list");
    let error = parse_page_response(
        &page(INDOOR_LIST, INDOOR_SEASON, "m"),
        &indoor_context(&catalog, "f"),
    )
    .expect_err("a boys page cannot enter a girls collection");
    match error {
        PageParseError::GenderMismatch { expected, actual } => {
            assert_eq!(expected, "f");
            assert_eq!(actual, "m");
        }
        other => panic!("unexpected page error: {other:?}"),
    }
}
