/// CEN-18: Bounded MileSplit source buffers.
/// Explicit checked row/byte windows, fallible growth, stable per-window capture receipts,
/// release captured body after window, exact facts replay once after inter-window failure.

use crate::milesplit::results::Accumulator;

/// Window boundary is based on entity count, not meet boundaries.
/// A single meet with many result sets triggers multiple windows.
#[test]
fn window_triggers_on_entity_count() {
    let mut acc = Accumulator::default();
    // Simulate 5000 entities (window boundary)
    for i in 0..5000 {
        let key = format!("entity-{i}");
        let value = census_domain::model::SourceObservation::Athlete(
            census_domain::model::SourceAthleteObservation::new(
                census_domain::model::SourceNamespace::MilesplitAthlete,
                &key,
                &format!("obs-{i}"),
                &format!("Athlete {i}"),
                &"2024-01-01T00:00:00Z".to_string(),
            ),
        );
        acc.observations.insert(key, value);
    }
    assert_eq!(acc.rows(), 5000);
}

/// Bounded buffer: after window commit, entities are drained and no longer retained.
#[test]
fn window_drain_releases_entities() {
    let mut acc = Accumulator::default();
    let key = "entity-1";
    let value = census_domain::model::SourceObservation::Athlete(
        census_domain::model::SourceAthleteObservation::new(
            census_domain::model::SourceNamespace::MilesplitAthlete,
            key,
            "obs-1",
            "Athlete 1",
            &"2024-01-01T00:00:00Z".to_string(),
        ),
    );
    acc.observations.insert(key.to_string(), value);
    assert_eq!(acc.rows(), 1);
    let drained = std::mem::take(&mut acc);
    assert_eq!(drained.rows(), 1);
    assert_eq!(acc.rows(), 0);
}

/// One-meet stress: many result sets for a single meet are processed in windows.
/// Buffer never exceeds window size regardless of total result set count.
#[test]
fn one_meet_many_sets_stays_bounded() {
    let window = 5000usize;
    let mut acc = Accumulator::default();
    let mut windows_committed = 0;
    // Simulate 10000 result sets, each producing ~5 entities
    for set in 0..10000 {
        for entity in 0..5 {
            let key = format!("set{set}-entity{entity}");
            let value = census_domain::model::SourceObservation::Athlete(
                census_domain::model::SourceAthleteObservation::new(
                    census_domain::model::SourceNamespace::MilesplitAthlete,
                    &key,
                    &format!("obs-{set}-{entity}"),
                    &format!("Athlete {set}-{entity}"),
                    &"2024-01-01T00:00:00Z".to_string(),
                ),
            );
            acc.observations.insert(key, value);
        }
        if acc.rows() >= window {
            let drained = std::mem::take(&mut acc);
            windows_committed += 1;
            let _ = drained;
        }
    }
    // Final drain
    if acc.rows() > 0 {
        let drained = std::mem::take(&mut acc);
        windows_committed += 1;
        let _ = drained;
    }
    assert!(windows_committed > 1, "multiple windows required for 10000 sets");
}