use std::collections::HashSet;
use std::path::Path;

use calamine::Reader;

use super::TestResult;

pub(super) fn xlsx_rows(path: &Path, sheet: &str) -> TestResult<usize> {
    let mut book = calamine::open_workbook_auto(path)?;
    let range = book.worksheet_range(sheet)?;
    Ok(range
        .rows()
        .filter(|row| {
            row.iter()
                .any(|cell| !matches!(cell, calamine::Data::Empty))
        })
        .count())
}

pub(super) fn unique_athlete_ids(path: &Path) -> TestResult<HashSet<String>> {
    let mut book = calamine::open_workbook_auto(path)?;
    let range = book.worksheet_range("Athletes")?;
    let mut ids = HashSet::new();
    for row in range.rows().skip(1) {
        if let Some(calamine::Data::String(id)) = row.first() {
            if !id.is_empty() {
                ids.insert(id.clone());
            }
        }
    }
    Ok(ids)
}
