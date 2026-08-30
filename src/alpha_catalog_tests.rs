use crate::alpha_catalog::{parse_nav_targets, ALLOWED_STATES};
use crate::alpha_model::{EventSpec, RunMatrix, StateTarget};
use crate::alpha_model_raw::{RawNavEvent, RawNavInfoResponse, RawNavState};

// ── Helpers ──────────────────────────────────────────────────────────

fn state(code: &str, id: u64) -> StateTarget {
    StateTarget {
        code: code.to_string(),
        state_id: id,
    }
}

fn event(short: &str, higher: bool) -> EventSpec {
    EventSpec {
        event_short: short.to_string(),
        higher_is_better: higher,
    }
}

// ── ALLOWED_STATES count ─────────────────────────────────────────────

#[test]
fn test_allowed_states_count_is_50() {
    assert_eq!(ALLOWED_STATES.len(), 50);
}

// ── from_targets: success ────────────────────────────────────────────

#[test]
fn test_all_50_states_accepted() {
    let states: Vec<StateTarget> = ALLOWED_STATES
        .iter()
        .enumerate()
        .map(|(i, code)| state(code, (i as u64 + 1) * 10))
        .collect();
    let seasons = vec![2024, 2025];
    let genders = vec!["M".to_string(), "F".to_string()];
    let events = vec![event("100m", false)];

    let matrix = RunMatrix::from_targets(states, seasons, genders, events).unwrap();
    // 50 states × 2 seasons × 2 genders × 1 event = 200
    assert_eq!(matrix.all().len(), 50 * 2 * 2 * 1);
}

#[test]
fn test_from_targets_cardinality() {
    let states = vec![state("CA", 1), state("TX", 2), state("NY", 3)];
    let seasons = vec![2023, 2024];
    let genders = vec!["M".to_string(), "F".to_string()];
    let events = vec![event("100m", false), event("long_jump", true), event("shot_put", true)];

    let matrix = RunMatrix::from_targets(states, seasons, genders, events).unwrap();
    let expected = 3 * 2 * 2 * 3;
    assert_eq!(matrix.all().len(), expected);
}

// ── from_targets: validation errors ──────────────────────────────────

#[test]
fn test_duplicate_state_rejected() {
    let states = vec![state("CA", 1), state("CA", 2)];
    assert!(RunMatrix::from_targets(states, vec![2024], vec!["M".into()], vec![event("100m", false)]).is_err());
}

#[test]
fn test_unknown_state_rejected() {
    let states = vec![state("ZZ", 99)];
    assert!(RunMatrix::from_targets(states, vec![2024], vec!["M".into()], vec![event("100m", false)]).is_err());
}

#[test]
fn test_zero_state_id_rejected() {
    let states = vec![state("CA", 0)];
    assert!(RunMatrix::from_targets(states, vec![2024], vec!["M".into()], vec![event("100m", false)]).is_err());
}

#[test]
fn test_dc_rejected() {
    let states = vec![state("DC", 1)];
    assert!(RunMatrix::from_targets(states, vec![2024], vec!["M".into()], vec![event("100m", false)]).is_err());
}

#[test]
fn test_negative_season_rejected() {
    let states = vec![state("CA", 1)];
    assert!(RunMatrix::from_targets(states, vec![-1], vec!["M".into()], vec![event("100m", false)]).is_err());
}

#[test]
fn test_empty_genders_rejected() {
    let states = vec![state("CA", 1)];
    assert!(RunMatrix::from_targets(states, vec![2024], vec!["".to_string()], vec![event("100m", false)]).is_err());
}

#[test]
fn test_empty_events_rejected() {
    let states = vec![state("CA", 1)];
    assert!(RunMatrix::from_targets(states, vec![2024], vec!["M".into()], vec![event("", false)]).is_err());
}

#[test]
fn test_duplicate_event_short_rejected() {
    let states = vec![state("CA", 1)];
    let events = vec![event("100m", false), event("100m", true)];
    assert!(RunMatrix::from_targets(states, vec![2024], vec!["M".into()], events).is_err());
}

// ── from_targets: deterministic ordering ─────────────────────────────

#[test]
fn test_matrix_is_sorted_by_state_season_gender_event() {
    let states = vec![state("TX", 2), state("AL", 1), state("CA", 3)];
    let seasons = vec![2025, 2024];
    let genders = vec!["F".to_string(), "M".to_string()];
    let events = vec![event("shot_put", true), event("100m", false)];

    let matrix = RunMatrix::from_targets(states, seasons, genders, events).unwrap();
    let units = matrix.all();

    for i in 1..units.len() {
        let a = &units[i - 1];
        let b = &units[i];
        assert!(
            (a.state.code.as_str(), a.season_id, a.gender.as_str(), a.event.event_short.as_str())
                <= (b.state.code.as_str(), b.season_id, b.gender.as_str(), b.event.event_short.as_str()),
            "ordering violation at index {}",
            i
        );
    }
}

// ── from_targets: all / units / take ─────────────────────────────────

#[test]
fn test_all_and_units_are_same() {
    let matrix = RunMatrix { units: vec![] };
    assert!(std::ptr::eq(matrix.all().as_ptr(), matrix.units().as_ptr()));
}

#[test]
fn test_take_some_one() {
    let states = vec![state("CA", 1), state("TX", 2)];
    let matrix = RunMatrix::from_targets(states, vec![2024], vec!["M".into()], vec![event("100m", false)]).unwrap();
    let taken = matrix.take(Some(1));
    assert_eq!(taken.len(), 1);
    assert_eq!(taken[0].state.code, "CA");
    // Original order preserved
    assert_eq!(matrix.all().len(), 2);
}

#[test]
fn test_take_some_zero() {
    let matrix = RunMatrix::from_targets(
        vec![state("CA", 1)],
        vec![2024],
        vec!["M".into()],
        vec![event("100m", false)],
    )
    .unwrap();
    let taken = matrix.take(Some(0));
    assert!(taken.is_empty());
}

#[test]
fn test_take_none_returns_all() {
    let matrix = RunMatrix::from_targets(
        vec![state("CA", 1)],
        vec![2024],
        vec!["M".into()],
        vec![event("100m", false)],
    )
    .unwrap();
    let taken = matrix.take(None);
    assert_eq!(taken.len(), 1);
}

#[test]
fn test_take_exceeds_length_clamps() {
    let matrix = RunMatrix::from_targets(
        vec![state("CA", 1)],
        vec![2024],
        vec!["M".into()],
        vec![event("100m", false)],
    )
    .unwrap();
    let taken = matrix.take(Some(999));
    assert_eq!(taken.len(), 1);
}

#[test]
fn test_no_duplicate_units() {
    let states = vec![state("CA", 1), state("TX", 2)];
    let matrix = RunMatrix::from_targets(states, vec![2024], vec!["M".into()], vec![event("100m", false)]).unwrap();
    let seen: std::collections::HashSet<_> = matrix
        .all()
        .iter()
        .map(|u| (u.state.code.clone(), u.season_id, u.gender.clone(), u.event.event_short.clone()))
        .collect();
    assert_eq!(seen.len(), matrix.all().len());
}

// ── Event direction preservation ─────────────────────────────────────

#[test]
fn test_track_event_is_lower_is_better() {
    let matrix = RunMatrix::from_targets(
        vec![state("CA", 1)],
        vec![2024],
        vec!["M".into()],
        vec![event("100m", false)],
    )
    .unwrap();
    assert!(!matrix.all()[0].event.higher_is_better);
}

#[test]
fn test_field_events_are_higher_is_better() {
    let matrix = RunMatrix::from_targets(
        vec![state("CA", 1)],
        vec![2024],
        vec!["M".into()],
        vec![event("long_jump", true), event("shot_put", true)],
    )
    .unwrap();
    for u in matrix.all() {
        assert!(u.event.higher_is_better);
    }
}

#[test]
fn test_event_direction_through_matrix() {
    let states = vec![state("CA", 1)];
    let events = vec![event("100m", false), event("long_jump", true), event("shot_put", true)];
    let matrix = RunMatrix::from_targets(states, vec![2024], vec!["M".into()], events).unwrap();
    let directions: std::collections::HashSet<bool> = matrix.all().iter().map(|u| u.event.higher_is_better).collect();
    assert_eq!(directions.len(), 2, "should have both true and false directions");
}

// ── parse_nav_targets: state parsing ─────────────────────────────────

#[test]
fn test_parse_nav_states_basic() {
    let responses = vec![RawNavInfoResponse {
        state: Some(RawNavState {
            state_id: Some(1),
            state: Some("CA".to_string()),
            state_name: Some("California".to_string()),
        }),
        event: None,
        divisions: None,
        genders: None,
        complete: true,
        page: None,
    }];

    let (states, _events) = parse_nav_targets(responses).unwrap();
    assert_eq!(states.len(), 1);
    assert_eq!(states[0].code, "CA");
    assert_eq!(states[0].state_id, 1);
}

#[test]
fn test_parse_nav_events_basic() {
    let responses = vec![RawNavInfoResponse {
        state: None,
        event: Some(RawNavEvent {
            event_short: Some("100m".to_string()),
            event_name: Some("100 meters".to_string()),
        }),
        divisions: None,
        genders: None,
        complete: true,
        page: None,
    }];

    let (_states, events) = parse_nav_targets(responses).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_short, "100m");
    assert!(!events[0].higher_is_better);
}

#[test]
fn test_parse_nav_missing_state_id_rejected() {
    let responses = vec![RawNavInfoResponse {
        state: Some(RawNavState {
            state_id: None,
            state: Some("CA".to_string()),
            state_name: None,
        }),
        event: None,
        divisions: None,
        genders: None,
        complete: true,
        page: None,
    }];

    assert!(parse_nav_targets(responses).is_err());
}

#[test]
fn test_parse_nav_missing_event_short_rejected() {
    let responses = vec![RawNavInfoResponse {
        state: None,
        event: Some(RawNavEvent {
            event_short: None,
            event_name: Some("100m".to_string()),
        }),
        divisions: None,
        genders: None,
        complete: true,
        page: None,
    }];

    assert!(parse_nav_targets(responses).is_err());
}

#[test]
fn test_parse_nav_zero_state_id_rejected() {
    let responses = vec![RawNavInfoResponse {
        state: Some(RawNavState {
            state_id: Some(0),
            state: Some("CA".to_string()),
            state_name: None,
        }),
        event: None,
        divisions: None,
        genders: None,
        complete: true,
        page: None,
    }];

    assert!(parse_nav_targets(responses).is_err());
}

#[test]
fn test_parse_nav_empty_event_short_rejected() {
    let responses = vec![RawNavInfoResponse {
        state: None,
        event: Some(RawNavEvent {
            event_short: Some("".to_string()),
            event_name: None,
        }),
        divisions: None,
        genders: None,
        complete: true,
        page: None,
    }];

    assert!(parse_nav_targets(responses).is_err());
}

#[test]
fn test_parse_nav_filters_to_50_codes() {
    // Mix of valid state and DC
    let responses = vec![
        RawNavInfoResponse {
            state: Some(RawNavState {
                state_id: Some(1),
                state: Some("CA".to_string()),
                state_name: None,
            }),
            event: None,
            divisions: None,
            genders: None,
            complete: true,
            page: None,
        },
        RawNavInfoResponse {
            state: Some(RawNavState {
                state_id: Some(99),
                state: Some("DC".to_string()),
                state_name: Some("District of Columbia".to_string()),
            }),
            event: None,
            divisions: None,
            genders: None,
            complete: true,
            page: None,
        },
    ];

    let (states, _) = parse_nav_targets(responses).unwrap();
    let codes: Vec<&str> = states.iter().map(|s| s.code.as_str()).collect();
    assert!(!codes.contains(&"DC"), "DC must be filtered out");
    assert!(codes.contains(&"CA"));
}

#[test]
fn test_parse_nav_divisions_filtered() {
    // Divisions should not appear as states
    let responses = vec![RawNavInfoResponse {
        state: Some(RawNavState {
            state_id: Some(1),
            state: Some("DIV-A".to_string()),
            state_name: None,
        }),
        event: Some(RawNavEvent {
            event_short: Some("100m".to_string()),
            event_name: Some("100m".to_string()),
        }),
        divisions: None,
        genders: None,
        complete: true,
        page: None,
    }];

    let (states, _) = parse_nav_targets(responses).unwrap();
    let codes: Vec<&str> = states.iter().map(|s| s.code.as_str()).collect();
    assert!(!codes.contains(&"DIV-A"), "Division names must not be in states");
}

#[test]
fn test_parse_nav_deduplicates_states() {
    let responses = vec![
        RawNavInfoResponse {
            state: Some(RawNavState {
                state_id: Some(1),
                state: Some("CA".to_string()),
                state_name: None,
            }),
            event: None,
            divisions: None,
            genders: None,
            complete: true,
            page: None,
        },
        RawNavInfoResponse {
            state: Some(RawNavState {
                state_id: Some(1),
                state: Some("CA".to_string()),
                state_name: None,
            }),
            event: None,
            divisions: None,
            genders: None,
            complete: true,
            page: None,
        },
    ];

    let (states, _) = parse_nav_targets(responses).unwrap();
    assert_eq!(states.len(), 1);
}

#[test]
fn test_parse_nav_deduplicates_events() {
    let responses = vec![
        RawNavInfoResponse {
            state: None,
            event: Some(RawNavEvent {
                event_short: Some("100m".to_string()),
                event_name: None,
            }),
            divisions: None,
            genders: None,
            complete: true,
            page: None,
        },
        RawNavInfoResponse {
            state: None,
            event: Some(RawNavEvent {
                event_short: Some("100m".to_string()),
                event_name: None,
            }),
            divisions: None,
            genders: None,
            complete: true,
            page: None,
        },
    ];

    let (_, events) = parse_nav_targets(responses).unwrap();
    assert_eq!(events.len(), 1);
}

#[test]
fn test_parse_nav_conflicting_duplicate_id_rejected() {
    // Same StateID but different codes → conflict
    let responses = vec![
        RawNavInfoResponse {
            state: Some(RawNavState {
                state_id: Some(1),
                state: Some("CA".to_string()),
                state_name: None,
            }),
            event: None,
            divisions: None,
            genders: None,
            complete: true,
            page: None,
        },
        RawNavInfoResponse {
            state: Some(RawNavState {
                state_id: Some(1),
                state: Some("TX".to_string()),
                state_name: None,
            }),
            event: None,
            divisions: None,
            genders: None,
            page: None,
            complete: true,
        },
    ];

    assert!(parse_nav_targets(responses).is_err());
}

#[test]
fn test_parse_nav_event_direction_100m_lower() {
    let responses = vec![RawNavInfoResponse {
        state: None,
        event: Some(RawNavEvent {
            event_short: Some("100m".to_string()),
            event_name: Some("100 meters".to_string()),
        }),
        divisions: None,
        genders: None,
        complete: true,
        page: None,
    }];

    let (_, events) = parse_nav_targets(responses).unwrap();
    assert!(!events[0].higher_is_better, "100m should be lower-is-better");
}

#[test]
fn test_parse_nav_event_direction_long_jump_higher() {
    let responses = vec![RawNavInfoResponse {
        state: None,
        event: Some(RawNavEvent {
            event_short: Some("long_jump".to_string()),
            event_name: Some("Long Jump".to_string()),
        }),
        divisions: None,
        genders: None,
        complete: true,
        page: None,
    }];

    let (_, events) = parse_nav_targets(responses).unwrap();
    assert!(events[0].higher_is_better, "long_jump should be higher-is-better");
}

#[test]
fn test_parse_nav_event_direction_shot_put_higher() {
    let responses = vec![RawNavInfoResponse {
        state: None,
        event: Some(RawNavEvent {
            event_short: Some("shot_put".to_string()),
            event_name: Some("Shot Put".to_string()),
        }),
        divisions: None,
        genders: None,
        complete: true,
        page: None,
    }];

    let (_, events) = parse_nav_targets(responses).unwrap();
    assert!(events[0].higher_is_better, "shot_put should be higher-is-better");
}

#[test]
fn test_parse_nav_multiple_states_and_events() {
    let responses = vec![
        RawNavInfoResponse {
            state: Some(RawNavState {
                state_id: Some(1),
                state: Some("CA".to_string()),
                state_name: None,
            }),
            event: Some(RawNavEvent {
                event_short: Some("100m".to_string()),
                event_name: None,
            }),
            divisions: None,
            genders: None,
            complete: true,
            page: None,
        },
        RawNavInfoResponse {
            state: Some(RawNavState {
                state_id: Some(2),
                state: Some("TX".to_string()),
                state_name: None,
            }),
            event: Some(RawNavEvent {
                event_short: Some("long_jump".to_string()),
                event_name: None,
            }),
            divisions: None,
            genders: None,
            complete: true,
            page: None,
        },
        RawNavInfoResponse {
            state: Some(RawNavState {
                state_id: Some(3),
                state: Some("NY".to_string()),
                state_name: None,
            }),
            event: Some(RawNavEvent {
                event_short: Some("shot_put".to_string()),
                event_name: None,
            }),
            divisions: None,
            genders: None,
            complete: true,
            page: None,
        },
    ];

    let (states, events) = parse_nav_targets(responses).unwrap();
    assert_eq!(states.len(), 3);
    assert_eq!(events.len(), 3);
    // States should be sorted
    assert_eq!(states[0].code, "CA");
    assert_eq!(states[1].code, "NY");
    assert_eq!(states[2].code, "TX");
}

#[test]
fn test_parse_nav_empty_responses() {
    let responses: Vec<RawNavInfoResponse> = vec![];
    let (states, events) = parse_nav_targets(responses).unwrap();
    assert!(states.is_empty());
    assert!(events.is_empty());
}

#[test]
fn test_parse_nav_state_sorting() {
    let mut responses = vec![];
    let codes = ["WY", "AL", "MT", "CA", "AK"];
    for (i, code) in codes.iter().enumerate() {
        responses.push(RawNavInfoResponse {
            state: Some(RawNavState {
                state_id: Some((i as u64 + 1) * 10),
                state: Some(code.to_string()),
                state_name: None,
            }),
            event: None,
            divisions: None,
            genders: None,
            complete: true,
            page: None,
        });
    }

    let (states, _) = parse_nav_targets(responses).unwrap();
    let sorted: Vec<&str> = states.iter().map(|s| s.code.as_str()).collect();
    assert_eq!(sorted, vec!["AK", "AL", "CA", "MT", "WY"]);
}

#[test]
fn test_parse_nav_event_sorting() {
    let mut responses = vec![];
    let event_shorts = vec!["shot_put", "100m", "long_jump"];
    for short in &event_shorts {
        responses.push(RawNavInfoResponse {
            state: None,
            event: Some(RawNavEvent {
                event_short: Some(short.to_string()),
                event_name: None,
            }),
            divisions: None,
            genders: None,
            complete: true,
            page: None,
        });
    }

    let (_, events) = parse_nav_targets(responses).unwrap();
    let sorted: Vec<&str> = events.iter().map(|e| e.event_short.as_str()).collect();
    assert_eq!(sorted, vec!["100m", "long_jump", "shot_put"]);
}

#[test]
fn test_parse_nav_trims_whitespace() {
    let responses = vec![RawNavInfoResponse {
        state: Some(RawNavState {
            state_id: Some(1),
            state: Some(" CA ".to_string()),
            state_name: None,
        }),
        event: Some(RawNavEvent {
            event_short: Some(" 100m ".to_string()),
            event_name: None,
        }),
        divisions: None,
        genders: None,
        complete: true,
        page: None,
    }];

    let (states, events) = parse_nav_targets(responses).unwrap();
    assert_eq!(states[0].code, "CA");
    assert_eq!(events[0].event_short, "100m");
}

#[test]
fn test_matrix_cardinality_50_x_seasons_x_genders_x_events() {
    let states: Vec<StateTarget> = ALLOWED_STATES
        .iter()
        .enumerate()
        .map(|(i, code)| state(code, (i as u64 + 1) * 10))
        .collect();
    let seasons = vec![2023, 2024, 2025];
    let genders = vec!["M".to_string(), "F".to_string(), "X".to_string()];
    let events = vec![event("100m", false), event("long_jump", true)];

    let matrix = RunMatrix::from_targets(states, seasons, genders, events).unwrap();
    let expected = 50 * 3 * 3 * 2;
    assert_eq!(matrix.all().len(), expected);
}
