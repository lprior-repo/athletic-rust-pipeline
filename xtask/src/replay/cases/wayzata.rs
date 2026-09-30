use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::Result;
use census_crawl::wayzata;

pub(super) fn wayzata_schedule(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    let Some(year) = season_year(file) else {
        return unmapped("wayzata", file);
    };
    let rows = wayzata::schedule_rows(body, year)?;
    ensure_rows(file, rows.len(), "schedule rows")?;
    Ok(format!("schedule season={year} rows={}", rows.len()))
}

fn season_year(file: &str) -> Option<i16> {
    file.split(|ch: char| !ch.is_ascii_digit())
        .find(|part| part.len() == 4)
        .and_then(|part| part.parse().ok())
}
