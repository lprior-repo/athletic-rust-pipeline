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
fn test_allowed_states_count_is_50() {
    assert_eq!(ALLOWED_STATES.len(), 50);
}

#[test]
fn test_all_50_states_accepted() {
    let seasons = vec![2024, 2025];
    let genders = vec!["M".to_string(), "F".to_string()];
    let events = vec![event("100m", false)];
    let matrix = RunMatrix::from_targets(states_50(), seasons, genders, events).unwrap();
    assert_eq!(matrix.all().len(), 50 * 2 * 2 * 1);
}

#[test]
fn test_from_targets_cardinality() {
    let seasons = vec![2023, 2024];
    let genders = vec!["M".to_string(), "F".to_string()];
    let events = vec![event("100m", false), event("long_jump", true), event("shot_put", true)];
    let matrix = RunMatrix::from_targets(states_50(), seasons, genders, events).unwrap();
    assert_eq!(matrix.all().len(), 50 * 2 * 2 * 3);
}

#[test]
fn test_matrix_is_sorted_by_state_season_gender_event() {
    let seasons = vec![2025, 2024];
    let genders = vec!["F".to_string(), "M".to_string()];
    let events = vec![event("shot_put", true), event("100m", false)];
    let matrix = RunMatrix::from_targets(states_50(), seasons, genders, events).unwrap();
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
    let seasons = vec![2024];
    let genders = vec!["M".to_string()];
    let events = vec![event("100m", false)];
    let matrix = RunMatrix::from_targets(states_50(), seasons, genders, events).unwrap();
    let taken = matrix.take(Some(1));
    assert_eq!(taken.len(), 1);
    assert_eq!(taken[0].state.code, "AK");
    assert_eq!(matrix.all().len(), 50);
}

#[test]
fn test_take_some_zero() {
    let matrix = RunMatrix::from_targets(states_50(), vec![2024], vec!["M".into()], vec![event("100m", false)]).unwrap();
    assert!(matrix.take(Some(0)).is_empty());
}

#[test]
fn test_take_none_returns_all() {
    let matrix = RunMatrix::from_targets(states_50(), vec![2024], vec!["M".into()], vec![event("100m", false)]).unwrap();
    assert_eq!(matrix.take(None).len(), 50);
}

#[test]
fn test_take_exceeds_length_clamps() {
    let matrix = RunMatrix::from_targets(states_50(), vec![2024], vec!["M".into()], vec![event("100m", false)]).unwrap();
    assert_eq!(matrix.take(Some(999)).len(), 50);
}

#[test]
fn test_no_duplicate_units() {
    let matrix = RunMatrix::from_targets(states_50(), vec![2024], vec!["M".into()], vec![event("100m", false)]).unwrap();
    let seen: std::collections::HashSet<_> = matrix
        .all()
        .iter()
        .map(|u| (u.state.code.clone(), u.season_id, u.gender.clone(), u.event.event_short.clone()))
        .collect();
    assert_eq!(seen.len(), matrix.all().len());
}

#[test]
fn test_track_event_is_lower_is_better() {
    let matrix = RunMatrix::from_targets(states_50(), vec![2024], vec!["M".into()], vec![event("100m", false)]).unwrap();
    assert!(!matrix.all()[0].event.higher_is_better);
}

#[test]
fn test_field_events_are_higher_is_better() {
    let events = vec![event("long_jump", true), event("shot_put", true)];
    let matrix = RunMatrix::from_targets(states_50(), vec![2024], vec!["M".into()], events).unwrap();
    for u in matrix.all() {
        assert!(u.event.higher_is_better);
    }
}

#[test]
fn test_event_direction_through_matrix() {
    let events = vec![event("100m", false), event("long_jump", true), event("shot_put", true)];
    let matrix = RunMatrix::from_targets(states_50(), vec![2024], vec!["M".into()], events).unwrap();
    let directions: std::collections::HashSet<bool> = matrix.all().iter().map(|u| u.event.higher_is_better).collect();
    assert_eq!(directions.len(), 2, "should have both true and false directions");
}

#[test]
fn test_matrix_cardinality_50_x_seasons_x_genders_x_events() {
    let seasons = vec![2023, 2024, 2025];
    let genders = vec!["M".to_string(), "F".to_string(), "X".to_string()];
    let events = vec![event("100m", false), event("long_jump", true)];
    let matrix = RunMatrix::from_targets(states_50(), seasons, genders, events).unwrap();
    assert_eq!(matrix.all().len(), 50 * 3 * 3 * 2);
}
#[test]
fn test_300h_direction() {
    let matrix = RunMatrix::from_targets(states_50(), vec![2024], vec!["M".into()], vec![event("300h", false)]).unwrap();
    assert!(!matrix.all()[0].event.higher_is_better, "300h should be lower-is-better");
}

#[test]
fn test_timed_events_all_lower_is_better() {
    let timed = vec!["55m", "60m", "80m", "100m", "200m", "300m", "400m", "500m", "600m",
        "800m", "1000m", "1500m", "1600m", "2000m", "3000m", "3200m", "5000m", "10000m",
        "100h", "110h", "300h", "400h", "60h", "mile"];
    let matrix = RunMatrix::from_targets(states_50(), vec![2024], vec!["M".into()], timed.iter().map(|s| event(s, false)).collect()).expect("timed events should be valid");
    for u in matrix.all() {
        assert!(!u.event.higher_is_better, "{} should be lower-is-better", u.event.event_short);
    }
}
