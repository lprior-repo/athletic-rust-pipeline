//! The store's append counters and the HTTP cache snapshot.
//!
//! `store_counters` iterates the LSM-tree stats and prints one row per table plus the total
//! observations and on-disk byte count. `cache_block` walks the HTTP cache directory and counts
//! `.meta.json` response headers and `.body` payload sizes so a resumed run reports the full
//! cache rather than only what this process fetched.
//!
//! Both blocks use [`count_cell`] to format counters as Excel-friendly numeric cells when the
//! value fits in a `u32`, and as plain text otherwise.

use crate::report::{io_error, ReportResult};
use crate::workbook::cells::{row, Cell};
use census_store::Store;

/// The store's append counters: one row per table, then the totals the LSM tree reports.
pub(super) fn store_counters(store: &Store) -> ReportResult<Vec<Vec<Cell>>> {
    let stats = store.stats()?;
    let mut cells = vec![row!("Store table", "Observations")];
    for (table, count) in &stats.tables {
        cells.push(row!(Cell::text(table.clone()), count_cell(*count)));
    }
    cells.push(row!("Total observations", count_cell(stats.observations)));
    cells.push(row!(
        "Store bytes on disk",
        Cell::text(stats.bytes_on_disk.to_string())
    ));
    Ok(cells)
}

/// What the HTTP cache holds: one entry per response already fetched, so a resumed run reports the
/// whole cache rather than only what this process fetched.
pub(super) fn cache_block(store: &Store) -> ReportResult<Vec<Vec<Cell>>> {
    let dir = store.http_cache_dir();
    let entries = std::fs::read_dir(&dir).map_err(|source| io_error(&dir, source))?;
    let mut responses = 0_u64;
    let mut bytes = 0_u64;
    for entry in entries {
        let entry = entry.map_err(|source| io_error(&dir, source))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.ends_with(".meta.json") {
            responses = responses.saturating_add(1);
        } else if name.ends_with(".body") {
            let size = entry
                .metadata()
                .map_err(|source| io_error(&dir, source))?
                .len();
            bytes = bytes.saturating_add(size);
        }
    }
    Ok(vec![
        row!("HTTP cache", "Value"),
        row!("Cached responses", count_cell(responses)),
        row!("Cached bytes", Cell::text(bytes.to_string())),
    ])
}

/// A counter as the number an Excel cell holds: exact through `u32`, and printed as text above it,
/// where a cell could no longer hold the value exactly.
fn count_cell(value: u64) -> Cell {
    match u32::try_from(value) {
        Ok(value) => Cell::Number(f64::from(value)),
        Err(_) => Cell::text(value.to_string()),
    }
}
