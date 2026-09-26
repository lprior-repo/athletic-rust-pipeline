//! The reduction itself: one pass over the store's own rows, and the row order it publishes.

use super::{is_relay, mark_text, sport_of, BestResult, Measure, Options};
use crate::report::{retain_core, retain_core_row, CoreScoped, Scope};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, EventKind,
};
use census_domain::MeetState;
use census_store::{Entity, Store, StoreResult, Table};
use std::collections::HashMap;

/// Reduce the consolidated tables to one best mark per `(athlete, event)`.
pub fn build(store: &Store, options: &Options) -> StoreResult<Vec<BestResult>> {
    let athletes: Vec<CanonicalAthlete> = scan_scoped(store, Table::Athletes, options.scope)?;
    let meets: Vec<CanonicalMeet> = scan_scoped(store, Table::Meets, options.scope)?;
    let events: Vec<CanonicalEvent> = scan_scoped(store, Table::Events, options.scope)?;

    let cohort: HashMap<&str, &CanonicalAthlete> =
        athletes.iter().map(|a| (a.id.as_str(), a)).collect();
    let meet_of: HashMap<&str, &CanonicalMeet> = meets.iter().map(|m| (m.id.as_str(), m)).collect();
    let kind_of: HashMap<&str, &EventKind> =
        events.iter().map(|e| (e.id.as_str(), &e.kind)).collect();

    let mut bests: HashMap<(String, String), BestResult> = HashMap::new();
    let mut counts: HashMap<(String, String), usize> = HashMap::new();
    store.for_each_merged(Table::Performances, |mut performance| {
        if options.scope == Scope::Core && !retain_core_row(&mut performance) {
            return Ok(());
        }
        accumulate(
            &mut bests,
            &mut counts,
            &performance,
            &cohort,
            &meet_of,
            &kind_of,
            options,
        );
        Ok(())
    })?;
    let mut rows = order_rows(bests, counts);
    if let Some(limit) = options.limit {
        rows.truncate(limit);
    }
    Ok(rows)
}
/// Sort bests into the published order: state → event → value (asc for times, desc otherwise)
/// → name → athlete, with marks-in-event attached from counts.
fn order_rows(
    bests: HashMap<(String, String), BestResult>,
    counts: HashMap<(String, String), usize>,
) -> Vec<BestResult> {
    let mut rows: Vec<BestResult> = bests
        .into_iter()
        .map(|(key, mut row)| {
            row.marks_in_event = counts.get(&key).copied().unwrap_or(1);
            row
        })
        .collect();
    rows.sort_by(|left, right| {
        left.state
            .cmp(&right.state)
            .then_with(|| left.event.cmp(&right.event))
            .then_with(|| {
                if left.measure == "time" {
                    left.best_value.cmp(&right.best_value)
                } else {
                    right.best_value.cmp(&left.best_value)
                }
            })
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.athlete_id.cmp(&right.athlete_id))
    });
    rows
}

/// Scan one table, keeping only the core scope's rows when that scope was asked for.
fn scan_scoped<T>(store: &Store, table: Table, scope: Scope) -> StoreResult<Vec<T>>
where
    T: Entity + CoreScoped,
{
    let mut rows: Vec<T> = store.scan(table)?;
    if scope == Scope::Core {
        retain_core(&mut rows);
    }
    Ok(rows)
}

/// Fold one performance into the `(athlete, event)` reductions, keeping the mark that wins.
fn accumulate(
    bests: &mut HashMap<(String, String), BestResult>,
    counts: &mut HashMap<(String, String), usize>,
    performance: &CanonicalPerformance,
    cohort: &HashMap<&str, &CanonicalAthlete>,
    meet_of: &HashMap<&str, &CanonicalMeet>,
    kind_of: &HashMap<&str, &EventKind>,
    options: &Options,
) {
    let Some(athlete) = cohort.get(performance.athlete.as_str()) else {
        return;
    };
    if let Some(year) = options.grad_year {
        if athlete.grad_year.get() != year {
            return;
        }
    }
    let Some(kind) = kind_of.get(performance.event.as_str()) else {
        return;
    };
    if is_relay(kind) {
        return;
    }
    let Some(measure) = Measure::of(&performance.mark) else {
        return;
    };
    let Some(value) = measure.value(&performance.mark) else {
        return;
    };
    let event_label = kind.stable_key().into_owned();
    let key = (athlete.id.as_str().to_string(), event_label.clone());
    let counter = counts.entry(key.clone()).or_insert(0);
    *counter = counter.saturating_add(1);

    let wins = bests
        .get(&key)
        .map(|incumbent| {
            let incumbent_value = incumbent.best_value;
            measure.better(value, incumbent_value)
        })
        .unwrap_or(true);
    if !wins {
        return;
    }
    let meet = meet_of.get(performance.meet.as_str()).copied();
    let row = best_row(
        athlete,
        kind,
        meet,
        performance,
        event_label,
        value,
        measure,
    );
    bests.insert(key, row);
}

/// The published row for one winning mark.
fn best_row(
    athlete: &CanonicalAthlete,
    kind: &EventKind,
    meet: Option<&CanonicalMeet>,
    performance: &CanonicalPerformance,
    event: String,
    value: i32,
    measure: Measure,
) -> BestResult {
    BestResult {
        athlete_id: athlete.id.as_str().to_string(),
        name: athlete.canonical_name.clone(),
        school: athlete.school.as_str().to_string(),
        state: MeetState::from(meet.and_then(|meet| meet.state)),
        grad_year: athlete.grad_year.get(),
        gender: athlete.gender.stable_key().to_string(),
        sport: sport_of(kind).to_string(),
        event,
        best_mark: mark_text(&performance.mark),
        best_value: value,
        measure: measure.as_str().to_string(),
        date: performance.date.clone(),
        meet: meet.map(|meet| meet.name.clone()).unwrap_or_default(),
        place: performance.place,
        wind_mps: performance.wind_mps,
        timing: performance
            .timing
            .map(|timing| timing.stable_key().to_string()),
        marks_in_event: 0,
        profile_url: athlete.public_profile_urls.first().cloned(),
    }
}
