
use std::collections::{HashMap, HashSet};

use census_domain::model::{CanonicalEvent, CanonicalPerformance};
use census_report::bests::mark_text;
use census_store::{Store, Table};

use super::compare::{field, Discrepancy, EntityCheck};

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
                message: format!("reading performances from store: {source}"),
            })?;
    let event_labels = event_labels(store)?;
    let lookup = performance_lookup(&performances, &event_labels);

    let mut passed: usize = 0;

    for &idx in sampled {
        let row = rows.get(idx).ok_or_else(|| Discrepancy {
            message: "row index out of range".to_string(),
        })?;
        check_performance_row(idx, row, &lookup, &event_labels, col_map)?;
        passed = passed.saturating_add(1);
    }

    Ok(EntityCheck {
        passed,
        sampled_indices: sampled.to_vec(),
    })
}

fn event_labels(store: &Store) -> Result<HashMap<String, String>, Discrepancy> {
    let events: Vec<CanonicalEvent> = store.scan(Table::Events).map_err(|source| Discrepancy {
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

fn event_label(event_id: &str, labels: &HashMap<String, String>) -> String {
    labels.get(event_id).cloned().unwrap_or_default()
}

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
    if let Some(label) = event_labels.get(event) {
        return Err(Discrepancy {
            message: format!(
                "performances row {idx}: id {aid} event '{event}' is the store's id for '{label}'; \
                 the sheet prints the label"
            ),
        });
    }
    Err(Discrepancy {
        message: format!(
            "performances row {idx}: id {aid} event '{event}' mark '{mark}' not in store"
        ),
    })
}
