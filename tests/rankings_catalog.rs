//! Navigation binding: the division list must come from the kind-specific
//! `seasons` entry, while the nav itself stays level-scoped.

use athletic_rust_pipeline::runtime::rankings::{EventCatalog, RequestedFamily, SeasonKind};

fn requested() -> Vec<RequestedFamily> {
    vec![RequestedFamily {
        group: "sprints".to_owned(),
        family: "100m".to_owned(),
        short: "100m".to_owned(),
    }]
}

fn nav(
    div_list_id: u64,
    level_div_id: u64,
    outdoor: u64,
    indoor: Option<u64>,
) -> serde_json::Value {
    let mut seasons = serde_json::json!({"2026": outdoor});
    if let Some(indoor) = indoor {
        seasons["12026"] = serde_json::json!(indoor);
    }
    serde_json::json!({
        "divListId": div_list_id,
        "levelDivId": level_div_id,
        "seasons": seasons,
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

#[test]
fn level_scoped_nav_selects_the_indoor_list_through_the_seasons_map() {
    let raw = nav(168_416, 168_416, 168_416, Some(173_005));
    let catalog = EventCatalog::from_nav(&raw, &requested(), SeasonKind::Indoor, 173_005)
        .expect("indoor nav");
    assert_eq!(catalog.list_id, 173_005);
    assert_eq!(catalog.level_div_id, 168_416);
    assert_eq!(catalog.season_id, 12_026);
    assert_eq!(catalog.families.len(), 1);
    assert!(catalog.absent_families.is_empty());
}

#[test]
fn outdoor_nav_selects_the_outdoor_list() {
    let raw = nav(168_416, 168_416, 168_416, Some(173_005));
    let catalog = EventCatalog::from_nav(&raw, &requested(), SeasonKind::Outdoor, 168_416)
        .expect("outdoor nav");
    assert_eq!(catalog.list_id, 168_416);
}

#[test]
fn seasons_map_must_point_at_the_requested_division() {
    // Requesting indoor while the seasons map still resolves `12026` to the
    // outdoor list cannot be reconciled into an indoor scope.
    let raw = nav(168_416, 168_416, 168_416, Some(168_416));
    let error = EventCatalog::from_nav(&raw, &requested(), SeasonKind::Indoor, 173_005)
        .expect_err("indoor request over an outdoor season entry");
    assert!(
        error
            .to_string()
            .contains("seasons['12026'] must select the requested division list 173005"),
        "unexpected error: {error}"
    );
}

#[test]
fn a_missing_season_entry_is_reported_with_its_key() {
    let raw = nav(168_416, 168_416, 168_416, None);
    let error = EventCatalog::from_nav(&raw, &requested(), SeasonKind::Indoor, 173_005)
        .expect_err("missing indoor season entry");
    assert!(
        error.to_string().contains("missing season 12026"),
        "unexpected error: {error}"
    );
}

#[test]
fn nav_outside_the_requested_level_is_rejected() {
    // The nav reports an unrelated division and level: no season entry can
    // reconcile it with the requested indoor list.
    let mut raw = nav(42, 43, 168_416, Some(173_005));
    raw["seasons"]["12026"] = serde_json::json!(173_005);
    let error = EventCatalog::from_nav(&raw, &requested(), SeasonKind::Indoor, 173_005)
        .expect_err("foreign division nav");
    assert!(
        error
            .to_string()
            .contains("nav covers division 42 outside level 43"),
        "unexpected error: {error}"
    );
}
