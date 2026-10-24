use super::invalid;
use crate::export::{ExportDataset, ReportedProfilePrTable};
use crate::report::ReportResult;
use crate::workbook::cells::Cell;
use census_crawl::participation_evidence::provider_target::profile_pr::SourceProfilePrV1;
use census_domain::model::CanonicalAthlete;
use std::collections::BTreeSet;

pub(super) fn cell(dataset: &ExportDataset, athlete: &CanonicalAthlete) -> ReportResult<Cell> {
    let lines = dataset
        .reported_profile_prs_for(athlete)
        .flat_map(|table| table.rows.iter().map(move |row| detailed_line(table, row)));
    if let Some(cell) = detailed(lines)? {
        return Ok(cell);
    }
    grouped(dataset, athlete)
}

fn detailed_line(table: &ReportedProfilePrTable, row: &SourceProfilePrV1) -> String {
    let locator = row.locator();
    format!(
        "{}: {}; Source-declared PR; Mode: {}; Level: {}; Published date: {}; Profile: {}; Capture: {}; Root: {}; Body interpretation: {}; Locator: {}:{}:{}",
        row.event(), row.published_mark(), row.source_mode(), row.source_level(),
        row.published_date(), table.profile_url, table.acquired_at.to_rfc3339(),
        table.root.key, table.interpretation, locator.ordinal(),
        locator.byte_offset(), locator.byte_length(),
    )
}

fn detailed(lines: impl Iterator<Item = String>) -> ReportResult<Option<Cell>> {
    let mut unique = BTreeSet::new();
    let mut units = 0_usize;
    for line in lines {
        if unique.contains(&line) {
            continue;
        }
        let needed = units
            .checked_add(line.encode_utf16().count())
            .and_then(|n| n.checked_add(usize::from(!unique.is_empty())))
            .ok_or_else(|| invalid("source-declared PR cell length overflow"))?;
        if needed > 32_767 {
            return Ok(None);
        }
        units = needed;
        unique.insert(line);
    }
    if unique.is_empty() {
        return Ok(Some(Cell::Empty));
    }
    Ok(Some(Cell::Text(join_detailed(unique)?)))
}

fn join_detailed(lines: BTreeSet<String>) -> ReportResult<String> {
    let mut text = String::new();
    for (index, line) in lines.into_iter().enumerate() {
        let separator = usize::from(index > 0);
        let bytes = line
            .len()
            .checked_add(separator)
            .ok_or_else(|| invalid("source-declared PR cell byte length overflow"))?;
        text.try_reserve(bytes)
            .map_err(|_| invalid("source-declared PR cell allocation failed"))?;
        if index > 0 {
            text.push('\n');
        }
        text.push_str(&line);
    }
    Ok(text)
}

fn grouped(dataset: &ExportDataset, athlete: &CanonicalAthlete) -> ReportResult<Cell> {
    let mut text = String::new();
    let mut units = 0_usize;
    for table in dataset.reported_profile_prs_for(athlete) {
        let header = format!(
            "Source-declared PR capture (unverified); Profile: {}; Capture: {}; Root: {}; Body interpretation: {}",
            table.profile_url, table.acquired_at.to_rfc3339(), table.root.key, table.interpretation,
        );
        append(&mut text, &mut units, &header)?;
        for row in &table.rows {
            let locator = row.locator();
            let line = format!(
                "{}: {}; Source-declared PR; Mode: {}; Level: {}; Published date: {}; Locator: {}:{}:{}",
                row.event(), row.published_mark(), row.source_mode(), row.source_level(),
                row.published_date(), locator.ordinal(), locator.byte_offset(), locator.byte_length(),
            );
            append(&mut text, &mut units, &line)?;
        }
    }
    Ok(Cell::Text(text))
}

fn append(text: &mut String, units: &mut usize, line: &str) -> ReportResult<()> {
    let separator = usize::from(!text.is_empty());
    let needed = units
        .checked_add(separator)
        .and_then(|n| n.checked_add(line.encode_utf16().count()))
        .ok_or_else(|| invalid("source-declared PR cell length overflow"))?;
    if needed > 32_767 {
        return Err(invalid(
            "complete grouped source-declared PR claims exceed Excel cell budget; no claims were truncated",
        ));
    }
    let bytes = line
        .len()
        .checked_add(separator)
        .ok_or_else(|| invalid("source-declared PR cell byte length overflow"))?;
    text.try_reserve(bytes)
        .map_err(|_| invalid("source-declared PR cell allocation failed"))?;
    if separator > 0 {
        text.push('\n');
    }
    text.push_str(line);
    *units = needed;
    Ok(())
}

#[cfg(test)]
mod tests;
