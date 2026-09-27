
use crate::report::{io_error, ReportResult};
use crate::workbook::cells::{row, Cell};
use census_store::Store;

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

fn count_cell(value: u64) -> Cell {
    match u32::try_from(value) {
        Ok(value) => Cell::Number(f64::from(value)),
        Err(_) => Cell::text(value.to_string()),
    }
}
