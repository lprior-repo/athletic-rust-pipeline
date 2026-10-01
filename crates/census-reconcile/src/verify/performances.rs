use std::collections::{HashMap, HashSet};

use census_report::export::ExportDataset;
use census_report::report::{Derivation, Scope};
use census_report::workbook::{PerformanceProjection, PerformanceRow};
use census_store::Store;

use super::columns::PERFORMANCES_REQUIRED;
use super::compare::{Discrepancy, EntityCheck};

pub fn verify_performances<T: AsRef<[String]>>(
    store: &Store,
    rows: &[T],
    sampled: &[usize],
    col_map: &HashMap<&str, usize>,
    scope: Scope,
) -> Result<EntityCheck, Discrepancy> {
    validate_columns(col_map)?;
    unique_result_ids(rows, col_map)?;
    let dataset = ExportDataset::load(store).map_err(|source| Discrepancy {
        message: format!("reading immutable verification input: {source}"),
    })?;
    let derivation = Derivation::of(&dataset, scope, None);
    let lookup: HashMap<&str, _> = derivation
        .performances()
        .iter()
        .map(|performance| (performance.id.as_str(), *performance))
        .collect();
    let projection = PerformanceProjection::of(&derivation);
    sampled.iter().try_for_each(|&idx| {
        let row = rows.get(idx).ok_or_else(|| Discrepancy {
            message: format!("performances row {idx}: sample index out of range"),
        })?;
        let row = row.as_ref();
        let id = cell(col_map, row, "Canonical Result ID", idx)?;
        let performance = lookup.get(id).ok_or_else(|| Discrepancy {
            message: format!("performances row {idx}: result {id} absent from {scope:?} input"),
        })?;
        compare_row(idx, row, col_map, &projection.row(performance))
    })?;
    Ok(EntityCheck {
        passed: sampled.len(),
        sampled_indices: sampled.to_vec(),
    })
}

fn validate_columns(col_map: &HashMap<&str, usize>) -> Result<(), Discrepancy> {
    PERFORMANCES_REQUIRED.iter().try_for_each(|name| {
        if col_map.contains_key(name) {
            Ok(())
        } else {
            Err(Discrepancy {
                message: format!("performances: missing required column {name}"),
            })
        }
    })
}

fn unique_result_ids<T: AsRef<[String]>>(
    rows: &[T],
    col_map: &HashMap<&str, usize>,
) -> Result<(), Discrepancy> {
    rows.iter()
        .enumerate()
        .try_fold(HashSet::new(), |mut seen, (idx, row)| {
            let id = cell(col_map, row.as_ref(), "Canonical Result ID", idx)?;
            if id.is_empty() || !seen.insert(id) {
                return Err(Discrepancy {
                    message: format!("performances row {idx}: empty or duplicate result ID {id}"),
                });
            }
            Ok(seen)
        })
        .map(|_| ())
}

fn compare_row(
    idx: usize,
    row: &[String],
    col_map: &HashMap<&str, usize>,
    expected: &PerformanceRow,
) -> Result<(), Discrepancy> {
    PERFORMANCES_REQUIRED
        .iter()
        .zip(expected.values())
        .try_for_each(|(name, expected)| {
            let actual = cell(col_map, row, name, idx)?;
            if expected.matches(actual) {
                Ok(())
            } else {
                Err(Discrepancy {
                    message: format!(
                        "performances row {idx}: {name} {actual:?} differs from {expected:?}"
                    ),
                })
            }
        })
}

fn cell<'a>(
    col_map: &HashMap<&str, usize>,
    row: &'a [String],
    name: &str,
    idx: usize,
) -> Result<&'a str, Discrepancy> {
    col_map
        .get(name)
        .and_then(|&column| row.get(column))
        .map(String::as_str)
        .ok_or_else(|| Discrepancy {
            message: format!("performances row {idx}: missing cell for {name}"),
        })
}

#[cfg(test)]
mod tests;
