use crate::report::{Census, ReportResult};
use crate::workbook::cells::{row, Cell};

use super::super::queues::SCHOOL_IDENTITY;
use super::super::{Family, StoreRows};

pub(super) fn reconciliation(
    rows: &StoreRows,
    conflicts: &[Family],
    census: &Census,
) -> ReportResult<Vec<Vec<Cell>>> {
    let mut cells = vec![row!("Reconciled counter", "Sheet rows", "Census", "Status")];
    append_population(&mut cells, rows, census)?;
    append_cohort(&mut cells, rows, census)?;
    append_source_findings(&mut cells, rows, conflicts, census)?;
    Ok(cells)
}

fn append_population(
    cells: &mut Vec<Vec<Cell>>,
    rows: &StoreRows,
    census: &Census,
) -> ReportResult<()> {
    let sheet_counts = [
        ("Schools", rows.schools.len(), census.totals.schools),
        ("Meets", rows.meets.len(), census.meets.total),
        ("Athletes", rows.athletes.len(), census.totals.athletes),
        ("Coaches", rows.coaches.len(), census.totals.coaches),
        (
            "Coaches with a published email",
            super::counters::count_coaches_with_email(rows.coaches),
            census.totals.coaches_with_email,
        ),
    ];
    append_counts(cells, sheet_counts)
}

fn append_cohort(
    cells: &mut Vec<Vec<Cell>>,
    rows: &StoreRows,
    census: &Census,
) -> ReportResult<()> {
    let sheet_counts = [
        (
            "Class-of-2027 athletes",
            super::counters::count_co2027(rows.athletes),
            census.totals.class_of_2027,
        ),
        (
            "Class-of-2027 athletes with grade evidence",
            super::counters::count_grade_evidence(rows.athletes),
            census.totals.class_of_2027_with_grad_year_evidence,
        ),
    ];
    append_counts(cells, sheet_counts)
}

fn append_source_findings(
    cells: &mut Vec<Vec<Cell>>,
    rows: &StoreRows,
    conflicts: &[Family],
    census: &Census,
) -> ReportResult<()> {
    let sheet_counts = [
        (
            "Meets naming an Athletic.net id",
            super::counters::count_athletic_net_meets(rows.meets),
            census.meets.with_athletic_net_id,
        ),
        (
            "Schools sharing a normalized name",
            findings_of(conflicts, SCHOOL_IDENTITY),
            census.duplicate_school_names,
        ),
    ];
    append_counts(cells, sheet_counts)
}

fn append_counts<const N: usize>(
    cells: &mut Vec<Vec<Cell>>,
    counts: [(&str, usize, usize); N],
) -> ReportResult<()> {
    for (label, sheet, census) in counts {
        cells.push(reconciled(label, sheet, census)?);
    }
    Ok(())
}

fn reconciled(label: &str, sheet: usize, census: usize) -> ReportResult<Vec<Cell>> {
    let status = if sheet == census {
        "reconciled"
    } else {
        "DIFFERS"
    };
    Ok(row!(
        Cell::text(label),
        Cell::number(sheet)?,
        Cell::number(census)?,
        Cell::text(status)
    ))
}

fn findings_of(families: &[Family], label: &str) -> usize {
    families
        .iter()
        .find(|family| family.label == label)
        .map_or(0, |family| family.findings)
}
