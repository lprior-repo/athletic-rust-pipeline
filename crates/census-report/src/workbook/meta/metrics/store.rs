use crate::export::{DatasetLineage, ExportDataset};
use crate::report::ReportResult;
use crate::workbook::cells::{row, Cell};

pub(super) fn frozen_counters(dataset: &ExportDataset) -> ReportResult<Vec<Vec<Cell>>> {
    let mut cells = lineage_cells(&dataset.lineage);
    append_revisions(&mut cells, &dataset.lineage);
    cells.push(row!("Frozen table", "Canonical records"));
    append_table_counts(&mut cells, dataset)?;
    Ok(cells)
}

fn lineage_cells(lineage: &DatasetLineage) -> Vec<Vec<Cell>> {
    vec![
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
    ]
}

fn append_revisions(cells: &mut Vec<Vec<Cell>>, lineage: &DatasetLineage) {
    cells.push(row!(
        "Export schema revision",
        Cell::Number(f64::from(lineage.schema_revision))
    ));
    cells.push(row!(
        "Export policy revision",
        Cell::Number(f64::from(lineage.policy_revision))
    ));
}

fn append_table_counts(cells: &mut Vec<Vec<Cell>>, dataset: &ExportDataset) -> ReportResult<()> {
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
    Ok(())
}
