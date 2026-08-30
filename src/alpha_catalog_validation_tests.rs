use crate::alpha_model::{EventSpec, RunMatrix, StateTarget};

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
