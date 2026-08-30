use crate::alpha_model_raw::{RawNavEvent, RawNavInfoResponse, RawNavState};
use crate::alpha_catalog::{parse_nav_targets, ALLOWED_STATES};

// ── parse_nav_targets: state parsing ─────────────────────────────────

#[test]
fn test_parse_nav_states_basic() {
    let responses = vec![RawNavInfoResponse {
        state: Some(RawNavState {
            state_id: Some(1),
            state: Some("CA".to_string()),
            state_name: Some("California".to_string()),
        }),
        event: None, divisions: None, genders: None, complete: true, page: None,
    }];
    let (states, _) = parse_nav_targets(responses).unwrap();
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
        divisions: None, genders: None, complete: true, page: None,
    }];
    let (_, events) = parse_nav_targets(responses).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_short, "100m");
    assert!(!events[0].higher_is_better);
}

#[test]
fn test_parse_nav_missing_state_id_rejected() {
    let responses = vec![RawNavInfoResponse {
        state: Some(RawNavState { state_id: None, state: Some("CA".to_string()), state_name: None }),
        event: None, divisions: None, genders: None, complete: true, page: None,
    }];
    assert!(parse_nav_targets(responses).is_err());
}

#[test]
fn test_parse_nav_missing_event_short_rejected() {
    let responses = vec![RawNavInfoResponse {
        state: None,
        event: Some(RawNavEvent { event_short: None, event_name: Some("100m".to_string()) }),
        divisions: None, genders: None, complete: true, page: None,
    }];
    assert!(parse_nav_targets(responses).is_err());
}

#[test]
fn test_parse_nav_zero_state_id_rejected() {
    let responses = vec![RawNavInfoResponse {
        state: Some(RawNavState { state_id: Some(0), state: Some("CA".to_string()), state_name: None }),
        event: None, divisions: None, genders: None, complete: true, page: None,
    }];
    assert!(parse_nav_targets(responses).is_err());
}

#[test]
fn test_parse_nav_empty_event_short_rejected() {
    let responses = vec![RawNavInfoResponse {
        state: None,
        event: Some(RawNavEvent { event_short: Some("".to_string()), event_name: None }),
        divisions: None, genders: None, complete: true, page: None,
    }];
    assert!(parse_nav_targets(responses).is_err());
}

#[test]
fn test_parse_nav_filters_to_50_codes() {
    let responses = vec![
        RawNavInfoResponse { state: Some(RawNavState { state_id: Some(1), state: Some("CA".to_string()), state_name: None }), event: None, divisions: None, genders: None, complete: true, page: None },
        RawNavInfoResponse { state: Some(RawNavState { state_id: Some(99), state: Some("DC".to_string()), state_name: Some("District of Columbia".to_string()) }), event: None, divisions: None, genders: None, complete: true, page: None },
    ];
    let (states, _) = parse_nav_targets(responses).unwrap();
    let codes: Vec<&str> = states.iter().map(|s| s.code.as_str()).collect();
    assert!(!codes.contains(&"DC"), "DC must be filtered out");
    assert!(codes.contains(&"CA"));
}

#[test]
fn test_parse_nav_divisions_filtered() {
    let responses = vec![RawNavInfoResponse {
        state: Some(RawNavState { state_id: Some(1), state: Some("DIV-A".to_string()), state_name: None }),
        event: Some(RawNavEvent { event_short: Some("100m".to_string()), event_name: Some("100m".to_string()) }),
        divisions: None, genders: None, complete: true, page: None,
    }];
    let (states, _) = parse_nav_targets(responses).unwrap();
    let codes: Vec<&str> = states.iter().map(|s| s.code.as_str()).collect();
    assert!(!codes.contains(&"DIV-A"), "Division names must not be in states");
}

#[test]
fn test_parse_nav_deduplicates_states() {
    let responses = vec![
        RawNavInfoResponse { state: Some(RawNavState { state_id: Some(1), state: Some("CA".to_string()), state_name: None }), event: None, divisions: None, genders: None, complete: true, page: None },
        RawNavInfoResponse { state: Some(RawNavState { state_id: Some(1), state: Some("CA".to_string()), state_name: None }), event: None, divisions: None, genders: None, complete: true, page: None },
    ];
    let (states, _) = parse_nav_targets(responses).unwrap();
    assert_eq!(states.len(), 1);
}

#[test]
fn test_parse_nav_deduplicates_events() {
    let responses = vec![
        RawNavInfoResponse { state: None, event: Some(RawNavEvent { event_short: Some("100m".to_string()), event_name: None }), divisions: None, genders: None, complete: true, page: None },
        RawNavInfoResponse { state: None, event: Some(RawNavEvent { event_short: Some("100m".to_string()), event_name: None }), divisions: None, genders: None, complete: true, page: None },
    ];
    let (_, events) = parse_nav_targets(responses).unwrap();
    assert_eq!(events.len(), 1);
}

#[test]
fn test_parse_nav_conflicting_duplicate_id_rejected() {
    let responses = vec![
        RawNavInfoResponse { state: Some(RawNavState { state_id: Some(1), state: Some("CA".to_string()), state_name: None }), event: None, divisions: None, genders: None, complete: true, page: None },
        RawNavInfoResponse { state: Some(RawNavState { state_id: Some(1), state: Some("TX".to_string()), state_name: None }), event: None, divisions: None, genders: None, complete: true, page: None },
    ];
    assert!(parse_nav_targets(responses).is_err());
}

#[test]
fn test_parse_nav_event_direction_100m_lower() {
    let responses = vec![RawNavInfoResponse {
        state: None,
        event: Some(RawNavEvent { event_short: Some("100m".to_string()), event_name: Some("100 meters".to_string()) }),
        divisions: None, genders: None, complete: true, page: None,
    }];
    let (_, events) = parse_nav_targets(responses).unwrap();
    assert!(!events[0].higher_is_better, "100m should be lower-is-better");
}

#[test]
fn test_parse_nav_event_direction_long_jump_higher() {
    let responses = vec![RawNavInfoResponse {
        state: None,
        event: Some(RawNavEvent { event_short: Some("long_jump".to_string()), event_name: Some("Long Jump".to_string()) }),
        divisions: None, genders: None, complete: true, page: None,
    }];
    let (_, events) = parse_nav_targets(responses).unwrap();
    assert!(events[0].higher_is_better, "long_jump should be higher-is-better");
}

#[test]
fn test_parse_nav_event_direction_shot_put_higher() {
    let responses = vec![RawNavInfoResponse {
        state: None,
        event: Some(RawNavEvent { event_short: Some("shot_put".to_string()), event_name: Some("Shot Put".to_string()) }),
        divisions: None, genders: None, complete: true, page: None,
    }];
    let (_, events) = parse_nav_targets(responses).unwrap();
    assert!(events[0].higher_is_better, "shot_put should be higher-is-better");
}

#[test]
fn test_parse_nav_multiple_states_and_events() {
    let responses = vec![
        RawNavInfoResponse { state: Some(RawNavState { state_id: Some(1), state: Some("CA".to_string()), state_name: None }), event: Some(RawNavEvent { event_short: Some("100m".to_string()), event_name: None }), divisions: None, genders: None, complete: true, page: None },
        RawNavInfoResponse { state: Some(RawNavState { state_id: Some(2), state: Some("TX".to_string()), state_name: None }), event: Some(RawNavEvent { event_short: Some("long_jump".to_string()), event_name: None }), divisions: None, genders: None, complete: true, page: None },
        RawNavInfoResponse { state: Some(RawNavState { state_id: Some(3), state: Some("NY".to_string()), state_name: None }), event: Some(RawNavEvent { event_short: Some("shot_put".to_string()), event_name: None }), divisions: None, genders: None, complete: true, page: None },
    ];
    let (states, events) = parse_nav_targets(responses).unwrap();
    assert_eq!(states.len(), 3);
    assert_eq!(events.len(), 3);
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
    for (i, code) in ["WY", "AL", "MT", "CA", "AK"].iter().enumerate() {
        responses.push(RawNavInfoResponse {
            state: Some(RawNavState { state_id: Some((i as u64 + 1) * 10), state: Some(code.to_string()), state_name: None }),
            event: None, divisions: None, genders: None, complete: true, page: None,
        });
    }
    let (states, _) = parse_nav_targets(responses).unwrap();
    let sorted: Vec<&str> = states.iter().map(|s| s.code.as_str()).collect();
    assert_eq!(sorted, vec!["AK", "AL", "CA", "MT", "WY"]);
}

#[test]
fn test_parse_nav_event_sorting() {
    let mut responses = vec![];
    for short in &["shot_put", "100m", "long_jump"] {
        responses.push(RawNavInfoResponse {
            state: None,
            event: Some(RawNavEvent { event_short: Some(short.to_string()), event_name: None }),
            divisions: None, genders: None, complete: true, page: None,
        });
    }
    let (_, events) = parse_nav_targets(responses).unwrap();
    let sorted: Vec<&str> = events.iter().map(|e| e.event_short.as_str()).collect();
    assert_eq!(sorted, vec!["100m", "long_jump", "shot_put"]);
}

#[test]
fn test_parse_nav_trims_whitespace() {
    let responses = vec![RawNavInfoResponse {
        state: Some(RawNavState { state_id: Some(1), state: Some(" CA ".to_string()), state_name: None }),
        event: Some(RawNavEvent { event_short: Some(" 100m ".to_string()), event_name: None }),
        divisions: None, genders: None, complete: true, page: None,
    }];
    let (states, events) = parse_nav_targets(responses).unwrap();
    assert_eq!(states[0].code, "CA");
    assert_eq!(events[0].event_short, "100m");
}
#[test]
fn test_parse_nav_exact_cardinality_50() {
    let responses: Vec<RawNavInfoResponse> = ALLOWED_STATES
        .iter()
        .enumerate()
        .map(|(i, code)| RawNavInfoResponse {
            state: Some(RawNavState { state_id: Some((i as u64 + 1) * 10), state: Some(code.to_string()), state_name: None }),
            event: None, divisions: None, genders: None, complete: true, page: None,
        })
        .collect();
    let (states, _) = parse_nav_targets(responses).unwrap();
    assert_eq!(states.len(), 50);
    let mut responses = vec![];
    for (i, code) in ["CA", "TX", "NY", "FL", "IL"].iter().enumerate() {
        responses.push(RawNavInfoResponse {
            state: Some(RawNavState { state_id: Some((i as u64 + 1) * 10), state: Some(code.to_string()), state_name: None }),
            event: None, divisions: None, genders: None, complete: true, page: None,
        });
    }
    for short in &["100m", "long_jump"] {
        responses.push(RawNavInfoResponse {
            state: None,
            event: Some(RawNavEvent { event_short: Some(short.to_string()), event_name: None }),
            divisions: None, genders: None, complete: true, page: None,
        });
    }
    let (states, events) = parse_nav_targets(responses).unwrap();
    assert_eq!(states.len(), 5);
    assert_eq!(events.len(), 2);
}

#[test]
fn test_parse_nav_110mh_direction() {
    let responses = vec![RawNavInfoResponse {
        state: None,
        event: Some(RawNavEvent { event_short: Some("110mh".to_string()), event_name: Some("110m hurdles".to_string()) }),
        divisions: None, genders: None, complete: true, page: None,
    }];
    let (_, events) = parse_nav_targets(responses).unwrap();
    assert!(!events[0].higher_is_better, "110mh should be lower-is-better");
}

#[test]
fn test_parse_nav_5k_direction() {
    let responses = vec![RawNavInfoResponse {
        state: None,
        event: Some(RawNavEvent { event_short: Some("5k".to_string()), event_name: Some("5000 meters".to_string()) }),
        divisions: None, genders: None, complete: true, page: None,
    }];
    let (_, events) = parse_nav_targets(responses).unwrap();
    assert!(!events[0].higher_is_better, "5k should be lower-is-better");
}
