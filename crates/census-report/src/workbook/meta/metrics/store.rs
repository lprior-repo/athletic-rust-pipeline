use crate::export::ExportDataset;
use crate::report::ReportResult;
use crate::workbook::cells::{row, Cell};

pub(super) fn frozen_counters(dataset: &ExportDataset) -> ReportResult<Vec<Vec<Cell>>> {
    let lineage = &dataset.lineage;
    let mut cells = vec![
        row!("Frozen input", "Value"),
        row!("Store identity", Cell::text(lineage.store_identity.clone())),
        row!(
            "Input generation",
            Cell::text(lineage.input_generation.clone())
        ),
        row!(
            "Input content digest",
            Cell::text(lineage.input_digest.clone())
        ),
        row!(
            "Source content digest",
            Cell::text(lineage.source_digest.clone())
        ),
        row!(
            "Captured store sequence",
            Cell::text(lineage.snapshot_sequence.to_string())
        ),
        row!(
            "Export schema revision",
            Cell::Number(f64::from(lineage.schema_revision))
        ),
        row!(
            "Export policy revision",
            Cell::Number(f64::from(lineage.policy_revision))
        ),
        row!("Frozen table", "Canonical records"),
    ];
    let counts = [
        ("athletes", dataset.athletes.len()),
        ("schools", dataset.schools.len()),
        ("teams", dataset.teams.len()),
        ("coaches", dataset.coaches.len()),
        ("coach observations", dataset.coach_observations.len()),
        ("events", dataset.events.len()),
        ("meets", dataset.meets.len()),
        ("performances", dataset.performances.len()),
        ("review cases", dataset.review_cases.len()),
        ("source access", dataset.source_access.len()),
        ("identity verdicts", dataset.verdicts.len()),
        ("identity decisions", dataset.identity_decisions.len()),
    ];
    counts.into_iter().try_for_each(|(table, count)| {
        cells.push(row!(table, Cell::number(count)?));
        Ok::<_, crate::report::ReportError>(())
    })?;
    Ok(cells)
}
