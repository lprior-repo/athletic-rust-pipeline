//! Row-level comparison against the store's canonical records.

use std::collections::{HashMap, HashSet};

use census_domain::model::{CanonicalAthlete, CanonicalPerformance};

use crate::store::{Store, Table};

/// A single discrepancy found during verification.
#[derive(Debug)]
pub struct Discrepancy {
    pub row: usize,
    pub message: String,
}

/// A verification pass over one entity type.
#[derive(Debug)]
pub struct EntityCheck {
    /// Total data rows read from the sheet(s).
    pub total_rows: usize,
    /// Number of rows that passed verification.
    pub passed: usize,
    /// Indices that were sampled.
    pub sampled_indices: Vec<usize>,
}

/// Extract a trimmed string field from a row via the column map.
fn field<'a>(col_map: &'a HashMap<&str, usize>, row: &'a [String], name: &str) -> &'a str {
    col_map
        .get(name)
        .and_then(|&i| row.get(i))
        .map(|s| s.trim())
        .unwrap_or("")
}

/// Verify athletes: every sampled row's athlete must exist in the store's Athletes table
/// with the same id, name, school id, and grad_year of 2027.
pub fn verify_athletes(
    store: &Store,
    rows: &[Vec<String>],
    sampled: &[usize],
    col_map: &HashMap<&str, usize>,
) -> Result<EntityCheck, Discrepancy> {
    let athletes: Vec<CanonicalAthlete> =
        store.scan(Table::Athletes).map_err(|source| Discrepancy {
            row: 0,
            message: format!("reading athletes from store: {source}"),
        })?;

    let school_ids: HashSet<String> = athletes
        .iter()
        .map(|a| a.school.as_str().to_string())
        .collect();

    let mut passed: usize = 0;

    for &idx in sampled {
        let row = rows.get(idx).ok_or_else(|| Discrepancy {
            row: idx,
            message: "row index out of range".to_string(),
        })?;
        check_athlete_row(idx, row, &athletes, &school_ids, col_map)?;
        passed = passed.saturating_add(1);
    }

    Ok(EntityCheck {
        total_rows: rows.len(),
        passed,
        sampled_indices: sampled.to_vec(),
    })
}

/// Check one sampled athlete row against the store.
fn check_athlete_row(
    idx: usize,
    row: &[String],
    athletes: &[CanonicalAthlete],
    school_ids: &HashSet<String>,
    col_map: &HashMap<&str, usize>,
) -> Result<(), Discrepancy> {
    let aid = field(col_map, row, "Athlete ID");
    let name = field(col_map, row, "Name");
    let school = field(col_map, row, "School");
    let grad_year = col_map
        .get("Graduation Year")
        .and_then(|&i| row.get(i))
        .and_then(|s| s.trim().parse::<i16>().ok());

    match athletes.iter().find(|a| a.id.as_str() == aid) {
        None => Err(Discrepancy {
            row: idx,
            message: format!("athletes row {idx}: id {aid} not in store"),
        }),
        Some(store_athlete) => {
            if store_athlete.canonical_name != name {
                return Err(Discrepancy {
                    row: idx,
                    message: format!(
                        "athletes row {idx}: id {aid} name '{workbook}' != store '{store}'",
                        workbook = name,
                        store = store_athlete.canonical_name,
                    ),
                });
            }
            if !school_ids.contains(school) {
                return Err(Discrepancy {
                    row: idx,
                    message: format!("athletes row {idx}: id {aid} school '{school}' not in store",),
                });
            }
            if grad_year != Some(2027) {
                return Err(Discrepancy {
                    row: idx,
                    message: format!(
                        "athletes row {idx}: id {aid} grad_year {} != 2027",
                        grad_year.map_or("?".to_string(), |y| y.to_string()),
                    ),
                });
            }
            Ok(())
        }
    }
}

/// Verify performances: every sampled row's (athlete id, event, mark) must exist in the store.
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

    // Build a lookup of (athlete_id, event_id, mark) for fast matching.
    let mut lookup: HashSet<(String, String, String)> = HashSet::new();
    for perf in &performances {
        let mark_str = match &perf.mark {
            census_domain::model::Mark::TimeSeconds(t) => format!("{t}"),
            census_domain::model::Mark::DistanceMetres(d) => format!("{d}"),
            census_domain::model::Mark::FieldImperial { feet_mark, .. } => feet_mark.clone(),
            census_domain::model::Mark::Points(p) => format!("{p}"),
            census_domain::model::Mark::Raw(r) => r.clone(),
        };
        lookup.insert((
            perf.athlete.as_str().to_string(),
            perf.event.as_str().to_string(),
            mark_str,
        ));
    }

    let mut passed: usize = 0;

    for &idx in sampled {
        let row = rows.get(idx).ok_or_else(|| Discrepancy {
            row: idx,
            message: "row index out of range".to_string(),
        })?;
        check_performance_row(idx, row, &lookup, col_map)?;
        passed = passed.saturating_add(1);
    }

    Ok(EntityCheck {
        total_rows: rows.len(),
        passed,
        sampled_indices: sampled.to_vec(),
    })
}

/// Check one sampled performance row against the store lookup.
fn check_performance_row(
    idx: usize,
    row: &[String],
    lookup: &HashSet<(String, String, String)>,
    col_map: &HashMap<&str, usize>,
) -> Result<(), Discrepancy> {
    let aid = field(col_map, row, "Athlete ID");
    let event = field(col_map, row, "Event");
    let mark = field(col_map, row, "Mark");

    let key = (aid.to_string(), event.to_string(), mark.to_string());
    if !lookup.contains(&key) {
        return Err(Discrepancy {
            row: idx,
            message: format!(
                "performances row {idx}: id {aid} event '{event}' mark '{mark}' not in store"
            ),
        });
    }
    Ok(())
}
