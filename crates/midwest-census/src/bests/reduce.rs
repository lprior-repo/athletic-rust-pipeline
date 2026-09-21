//! The reduction itself: one pass over the store's own rows, and the row order it publishes.

use super::{is_relay, mark_text, sport_of, BestResult, Measure, Options};
use crate::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, EventKind,
};
use crate::report::{retain_core, Scope};
use crate::store::{Store, Table};
use anyhow::Result;
use std::collections::HashMap;

/// Reduce the consolidated tables to one best mark per `(athlete, event)`.
pub fn build(store: &Store, options: &Options) -> Result<Vec<BestResult>> {
    // Read the store itself: the report reads the store too, and a snapshot left behind by an older
    // `consolidate` would silently disagree with it.
    let mut athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    let mut meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
    let mut events: Vec<CanonicalEvent> = store.scan(Table::Events)?;
    let mut performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    if options.scope == Scope::Core {
        retain_core(&mut athletes);
        retain_core(&mut meets);
        retain_core(&mut events);
        retain_core(&mut performances);
    }

    let cohort: HashMap<&str, &CanonicalAthlete> =
        athletes.iter().map(|a| (a.id.as_str(), a)).collect();
    let meet_of: HashMap<&str, &CanonicalMeet> = meets.iter().map(|m| (m.id.as_str(), m)).collect();
    let kind_of: HashMap<&str, &EventKind> =
        events.iter().map(|e| (e.id.as_str(), &e.kind)).collect();

    let mut bests: HashMap<(String, String), BestResult> = HashMap::new();
    let mut counts: HashMap<(String, String), usize> = HashMap::new();
    for performance in &performances {
        let Some(athlete) = cohort.get(performance.athlete.as_str()) else {
            continue;
        };
        if let Some(year) = options.grad_year {
            if athlete.grad_year.get() != year {
                continue;
            }
        }
        let Some(kind) = kind_of.get(performance.event.as_str()) else {
            continue;
        };
        if is_relay(kind) {
            continue;
        }
        let Some(measure) = Measure::of(&performance.mark) else {
            continue;
        };
        let Some(value) = measure.value(&performance.mark) else {
            continue;
        };
        let event_label = format!("{kind:?}");
        let key = (athlete.id.as_str().to_string(), event_label.clone());
        // A count is bounded by the scanned performance rows, so saturation is unreachable; it is
        // here so a change to that bound can never wrap the counter.
        let counter = counts.entry(key.clone()).or_insert(0);
        *counter = counter.saturating_add(1);

        let wins = bests
            .get(&key)
            .map(|incumbent| measure.better(value, incumbent.best_value))
            .unwrap_or(true);
        if !wins {
            continue;
        }
        let meet = meet_of.get(performance.meet.as_str());
        bests.insert(
            key,
            BestResult {
                athlete_id: athlete.id.as_str().to_string(),
                name: athlete.canonical_name.clone(),
                school: athlete.school.as_str().to_string(),
                state: meet
                    .map(|meet| meet.state.clone())
                    .unwrap_or_else(|| "??".to_string()),
                grad_year: athlete.grad_year.get(),
                gender: format!("{:?}", athlete.gender),
                sport: sport_of(kind).to_string(),
                event: event_label,
                best_mark: mark_text(&performance.mark),
                best_value: value,
                measure: measure.as_str().to_string(),
                date: performance.date.clone(),
                meet: meet.map(|meet| meet.name.clone()).unwrap_or_default(),
                place: performance.place,
                wind_mps: performance.wind_mps,
                timing: performance.timing.map(|timing| format!("{timing:?}")),
                marks_in_event: 0,
                profile_url: athlete.public_profile_urls.first().cloned(),
            },
        );
    }

    let mut rows: Vec<BestResult> = bests
        .into_iter()
        .map(|(key, mut row)| {
            row.marks_in_event = counts.get(&key).copied().unwrap_or(1);
            row
        })
        .collect();
    // Best first inside each event; times ascend, everything else descends.
    rows.sort_by(|left, right| {
        left.state
            .cmp(&right.state)
            .then_with(|| left.event.cmp(&right.event))
            .then_with(|| {
                if left.measure == "time" {
                    left.best_value.total_cmp(&right.best_value)
                } else {
                    right.best_value.total_cmp(&left.best_value)
                }
            })
            .then_with(|| left.name.cmp(&right.name))
    });
    if let Some(limit) = options.limit {
        rows.truncate(limit);
    }
    Ok(rows)
}
