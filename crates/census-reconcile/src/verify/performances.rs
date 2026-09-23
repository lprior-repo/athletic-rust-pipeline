//! The Performances sheets' row-level check: every sampled row's athlete, event and mark must
//! resolve the way the sheet prints them.
//!
//! The sheet prints the event by the label the census mints for its kind (`ShotPut`, `Track1600m`)
//! and the mark in the published notation ([`mark_text`]), so the check resolves both through the
//! store instead of comparing a printed label against a set of ids — the same rule the Athletes
//! sheet's School cell gets, for the same reason. An event the store holds no row for has no label
//! to print, and the sheet leaves that cell empty (`PerformanceRow::event_label`'s
//! `unwrap_or_default`) — the check resolves an empty cell the same way.

use std::collections::{HashMap, HashSet};

use census_domain::model::{CanonicalEvent, CanonicalPerformance};
use census_report::bests::mark_text;
use census_store::{Store, Table};

use super::compare::{field, Discrepancy, EntityCheck};

/// Verify performances: every sampled row's athlete, event and mark must resolve the way the sheet
/// prints them.
pub fn verify_performances(
    store: &Store,
    rows: &[Vec<String>],
    sampled: &[usize],
    col_map: &HashMap<&str, usize>,
) -> Result<EntityCheck, Discrepancy> {
    let performances: Vec<CanonicalPerformance> =
        store
            .scan(Table::Performances)
            .map_err(|source| Discrepancy {
                row: 0,
                message: format!("reading performances from store: {source}"),
            })?;
    let event_labels = event_labels(store)?;
    let lookup = performance_lookup(&performances, &event_labels);

    let mut passed: usize = 0;

    for &idx in sampled {
        let row = rows.get(idx).ok_or_else(|| Discrepancy {
            row: idx,
            message: "row index out of range".to_string(),
        })?;
        check_performance_row(idx, row, &lookup, &event_labels, col_map)?;
        passed = passed.saturating_add(1);
    }

    Ok(EntityCheck {
        total_rows: rows.len(),
        passed,
        sampled_indices: sampled.to_vec(),
    })
}

/// Every event row's printed label, by event id: the sheet writes `kind.stable_key()`.
fn event_labels(store: &Store) -> Result<HashMap<String, String>, Discrepancy> {
    let events: Vec<CanonicalEvent> =
        store
            .scan(Table::Events)
            .map_err(|source| Discrepancy {
                row: 0,
                message: format!("reading events from store: {source}"),
            })?;
    Ok(events
        .into_iter()
        .map(|event| {
            (
                event.id.as_str().to_string(),
                event.kind.stable_key().to_string(),
            )
        })
        .collect())
}

/// The lookup a sampled row is checked against: (athlete id, event label, published mark).
fn performance_lookup(
    performances: &[CanonicalPerformance],
    event_labels: &HashMap<String, String>,
) -> HashSet<(String, String, String)> {
    performances
        .iter()
        .map(|perf| {
            (
                perf.athlete.as_str().to_string(),
                event_label(perf.event.as_str(), event_labels),
                mark_text(&perf.mark),
            )
        })
        .collect()
}

/// The label the sheet prints for `event_id`: the event's kind label, or the empty cell the sheet
/// leaves when the store holds no event row.
fn event_label(event_id: &str, labels: &HashMap<String, String>) -> String {
    labels.get(event_id).cloned().unwrap_or_default()
}

/// Check one sampled performance row against the store lookup.
fn check_performance_row(
    idx: usize,
    row: &[String],
    lookup: &HashSet<(String, String, String)>,
    event_labels: &HashMap<String, String>,
    col_map: &HashMap<&str, usize>,
) -> Result<(), Discrepancy> {
    let aid = field(col_map, row, "Athlete ID");
    let event = field(col_map, row, "Event");
    let mark = field(col_map, row, "Mark");

    let key = (aid.to_string(), event.to_string(), mark.to_string());
    if lookup.contains(&key) {
        return Ok(());
    }
    // Name the store side where it exists: an Event cell carrying the id where the store holds a
    // label is this sheet's version of the school-id leak, and saying so beats "not in store".
    if let Some(label) = event_labels.get(event) {
        return Err(Discrepancy {
            row: idx,
            message: format!(
                "performances row {idx}: id {aid} event '{event}' is the store's id for '{label}'; \
                 the sheet prints the label"
            ),
        });
    }
    Err(Discrepancy {
        row: idx,
        message: format!(
            "performances row {idx}: id {aid} event '{event}' mark '{mark}' not in store"
        ),
    })
}
