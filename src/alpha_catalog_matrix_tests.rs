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

#[test]
fn test_allowed_states_count_is_50() {
    assert_eq!(ALLOWED_STATES.len(), 50);
}

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
    assert_eq!(matrix.all().len(), 50 * 2 * 2 * 1);
}

#[test]
fn test_from_targets_cardinality() {
    let states = vec![state("CA", 1), state("TX", 2), state("NY", 3)];
    let seasons = vec![2023, 2024];
    let genders = vec!["M".to_string(), "F".to_string()];
    let events = vec![event("100m", false), event("long_jump", true), event("shot_put", true)];
    let matrix = RunMatrix::from_targets(states, seasons, genders, events).unwrap();
    assert_eq!(matrix.all().len(), 3 * 2 * 2 * 3);
}

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
    assert_eq!(matrix.all().len(), 2);
}

#[test]
fn test_take_some_zero() {
    let matrix = RunMatrix::from_targets(
        vec![state("CA", 1)], vec![2024], vec!["M".into()], vec![event("100m", false)]
    ).unwrap();
    assert!(matrix.take(Some(0)).is_empty());
}

#[test]
fn test_take_none_returns_all() {
    let matrix = RunMatrix::from_targets(
        vec![state("CA", 1)], vec![2024], vec!["M".into()], vec![event("100m", false)]
    ).unwrap();
    assert_eq!(matrix.take(None).len(), 1);
}

#[test]
fn test_take_exceeds_length_clamps() {
    let matrix = RunMatrix::from_targets(
        vec![state("CA", 1)], vec![2024], vec!["M".into()], vec![event("100m", false)]
    ).unwrap();
    assert_eq!(matrix.take(Some(999)).len(), 1);
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

#[test]
fn test_track_event_is_lower_is_better() {
    let matrix = RunMatrix::from_targets(
        vec![state("CA", 1)], vec![2024], vec!["M".into()], vec![event("100m", false)]
    ).unwrap();
    assert!(!matrix.all()[0].event.higher_is_better);
}

#[test]
fn test_field_events_are_higher_is_better() {
    let matrix = RunMatrix::from_targets(
        vec![state("CA", 1)], vec![2024], vec!["M".into()],
        vec![event("long_jump", true), event("shot_put", true)]
    ).unwrap();
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
    assert_eq!(matrix.all().len(), 50 * 3 * 3 * 2);
}
