//! Census scope counters and method notes.
//!
//! [`SCOPE_COUNTERS`] enumerates the seven headline counters the census publishes for each scope
//! (core, all sources). [`scope_counters`] renders them side by side. [`method_notes`] prints any
//! publication method notes the census carries.

use crate::report::{Census, ReportResult};
use crate::workbook::cells::{row, Cell};

/// One census counter: the label a block prints and the counted field behind it.
type ScopeCounter = (&'static str, fn(&Census) -> usize);

/// The counted fields of both census scopes, in published order.
const SCOPE_COUNTERS: [ScopeCounter; 7] = [
    ("Schools", |census| census.totals.schools),
    ("Athletes", |census| census.totals.athletes),
    ("Coaches", |census| census.totals.coaches),
    ("Coaches with a published email", |census| {
        census.totals.coaches_with_email
    }),
    ("Class of 2027", |census| census.totals.class_of_2027),
    ("Meets", |census| census.meets.total),
    ("Meets naming an Athletic.net id", |census| {
        census.meets.with_athletic_net_id
    }),
];

/// Both census scopes' headline counters, side by side.
pub(super) fn scope_counters(core: &Census, all_sources: &Census) -> ReportResult<Vec<Vec<Cell>>> {
    let mut cells = vec![row!("Core", "Count", "All sources", "Count")];
    for (label, read) in SCOPE_COUNTERS {
        cells.push(row!(
            Cell::text(label),
            Cell::number(read(core))?,
            Cell::text(label),
            Cell::number(read(all_sources))?,
        ));
    }
    Ok(cells)
}

/// The census's publication method notes, one row each.
pub(super) fn method_notes(census: &Census) -> Vec<Vec<Cell>> {
    let mut cells = vec![row!("Method note", "Value")];
    cells.extend(
        census
            .notes
            .iter()
            .filter(|note| note.starts_with("core performance publication"))
            .map(|note| row!("Core performance publication", Cell::text(note.clone()))),
    );
    cells
}
