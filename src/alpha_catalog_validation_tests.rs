use crate::alpha_catalog::ALLOWED_STATES;
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

fn states_50() -> Vec<StateTarget> {
    ALLOWED_STATES
        .iter()
        .enumerate()
        .map(|(i, code)| state(code, (i as u64 + 1) * 10))
        .collect()
}

#[test]
fn test_exactly_50_states_required() {
    let mut states = states_50();
    states.pop();
    assert!(RunMatrix::from_targets(states, vec![2024], vec!["M".into()], vec![event("100m", false)]).is_err());
}

#[test]
fn test_too_many_states_rejected() {
    let mut states = states_50();
    states.push(state("ZZ", 99));
    assert!(RunMatrix::from_targets(states, vec![2024], vec!["M".into()], vec![event("100m", false)]).is_err());
}

#[test]
fn test_duplicate_state_code_rejected() {
    let mut states = states_50();
    states[0] = state("CA", 10); // replace first with CA, keeping 50
    assert!(RunMatrix::from_targets(states, vec![2024], vec!["M".into()], vec![event("100m", false)]).is_err());
}

#[test]
fn test_duplicate_state_id_rejected() {
    let mut states = states_50();
    states[1] = state("AK", 10); // keep AK, duplicate AL's ID (10), 50 entries
    assert!(RunMatrix::from_targets(states, vec![2024], vec!["M".into()], vec![event("100m", false)]).is_err());
}

#[test]
fn test_unknown_state_rejected() {
    let mut states = states_50();
    states.pop();
    let mut s = states;
    s.push(state("ZZ", 1000));
    assert!(RunMatrix::from_targets(s, vec![2024], vec!["M".into()], vec![event("100m", false)]).is_err());
}

#[test]
fn test_zero_state_id_rejected() {
    let mut states = states_50();
    states[0] = state("AL", 0);
    assert!(RunMatrix::from_targets(states, vec![2024], vec!["M".into()], vec![event("100m", false)]).is_err());
}

#[test]
fn test_dc_rejected() {
    let mut states = states_50();
    states.pop();
    let mut s = states;
    s.push(state("DC", 1));
    assert!(RunMatrix::from_targets(s, vec![2024], vec!["M".into()], vec![event("100m", false)]).is_err());
}

#[test]
fn test_negative_season_rejected() {
    assert!(RunMatrix::from_targets(states_50(), vec![-1], vec!["M".into()], vec![event("100m", false)]).is_err());
}

#[test]
fn test_zero_season_rejected() {
    assert!(RunMatrix::from_targets(states_50(), vec![0], vec!["M".into()], vec![event("100m", false)]).is_err());
}

#[test]
fn test_empty_genders_rejected() {
    assert!(RunMatrix::from_targets(states_50(), vec![2024], vec![], vec![event("100m", false)]).is_err());
}

#[test]
fn test_whitespace_gender_rejected() {
    assert!(RunMatrix::from_targets(states_50(), vec![2024], vec!["  ".to_string()], vec![event("100m", false)]).is_err());
}

#[test]
fn test_duplicate_genders_rejected() {
    assert!(RunMatrix::from_targets(states_50(), vec![2024], vec!["M".into(), "M".into()], vec![event("100m", false)]).is_err());
}

#[test]
fn test_empty_event_rejected() {
    assert!(RunMatrix::from_targets(states_50(), vec![2024], vec!["M".into()], vec![event("", false)]).is_err());
}

#[test]
fn test_whitespace_event_rejected() {
    assert!(RunMatrix::from_targets(states_50(), vec![2024], vec!["M".into()], vec![event("  ", false)]).is_err());
}

#[test]
fn test_duplicate_event_short_rejected() {
    let events = vec![event("100m", false), event("100m", true)];
    assert!(RunMatrix::from_targets(states_50(), vec![2024], vec!["M".into()], events).is_err());
}
#[test]
fn test_empty_seasons_rejected() {
    assert!(RunMatrix::from_targets(states_50(), vec![], vec!["M".into()], vec![event("100m", false)]).is_err());
}

#[test]
fn test_duplicate_seasons_rejected() {
    assert!(RunMatrix::from_targets(states_50(), vec![2024, 2024], vec!["M".into()], vec![event("100m", false)]).is_err());
}
