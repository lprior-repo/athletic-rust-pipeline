use super::invalid;
use crate::export::ExportDataset;
use crate::report::ReportResult;
use crate::workbook::cells::Cell;
use census_domain::model::CanonicalAthlete;
use std::collections::BTreeSet;

pub(super) fn cell(dataset: &ExportDataset, athlete: &CanonicalAthlete) -> ReportResult<Cell> {
    let mut lines = BTreeSet::new();
    let mut units = 0_usize;
    for table in dataset.reported_profile_prs_for(athlete) {
        for row in &table.rows {
            let locator = row.locator();
            let line = format!(
                "{}: {}; Source-declared PR; Mode: {}; Level: {}; Published date: {}; Profile: {}; Capture: {}; Root: {}; Body interpretation: {}; Locator: {}:{}:{}",
                row.event(), row.published_mark(), row.source_mode(), row.source_level(),
                row.published_date(), table.profile_url, table.acquired_at.to_rfc3339(),
                table.root.key, table.interpretation, locator.ordinal(),
                locator.byte_offset(), locator.byte_length(),
            );
            if lines.contains(&line) {
                continue;
            }
            units = units
                .checked_add(line.encode_utf16().count())
                .and_then(|n| n.checked_add(usize::from(!lines.is_empty())))
                .ok_or_else(|| invalid("source-declared PR cell length overflow"))?;
            if units > 32_767 {
                return Err(invalid(
                    "source-declared PR cell exceeds Excel's text limit",
                ));
            }
            lines.insert(line);
        }
    }
    if lines.is_empty() {
        Ok(Cell::Empty)
    } else {
        Ok(Cell::text(lines.into_iter().collect::<Vec<_>>().join("\n")))
    }
}
