use std::path::Path;

use crate::report::{Derivation, ReportResult};
use crate::workbook::verify::meta::{contacts, Expect};

use super::cells::Cell;
use super::defect;
use super::input::Inputs;
use super::read::{self, Limits};

pub(super) fn verify(directory: &Path, inputs: &Inputs<'_>) -> ReportResult<()> {
    let population = Derivation::of(inputs.dataset, inputs.options.scope, None);
    let [(_, mailboxes), (_, research)] =
        contacts::expected(population.schools(), inputs.school_year)?;
    verify_rows(&directory.join("school-contacts.csv"), mailboxes)?;
    verify_rows(&directory.join("contact-research.csv"), research)
}

fn verify_rows(path: &Path, expected: Vec<Vec<Expect>>) -> ReportResult<()> {
    let count = expected.len();
    let mut rows = expected.into_iter();
    let limits = Limits {
        bytes: 1024 * 1024 * 1024,
        records: count,
    };
    let total = read::csv(path, limits, "contact sidecar", |index, record| {
        let row = rows.next().ok_or_else(|| {
            defect(format!(
                "unexpected contact row {index}: {}",
                path.display()
            ))
        })?;
        compare_row(path, index, record, row)
    })?;
    if total != count {
        return Err(defect(format!(
            "{} retains {total} contact records, expected {count}",
            path.display()
        )));
    }
    Ok(())
}

fn compare_row(
    path: &Path,
    index: usize,
    record: &csv::StringRecord,
    expected: Vec<Expect>,
) -> ReportResult<()> {
    if record.len() != expected.len() {
        return Err(defect(format!(
            "contact row {index} has a different width: {}",
            path.display()
        )));
    }
    for (column, expected) in expected.into_iter().enumerate() {
        let cell = match expected {
            Expect::Text(value) => Cell::text(value),
            Expect::Empty => Cell::Text(None),
            Expect::Number(_) => {
                return Err(defect(
                    "contact schema contains an unexpected numeric field".to_owned(),
                ))
            }
        };
        let found = record
            .get(column)
            .ok_or_else(|| defect("contact cell is absent".to_owned()))?;
        cell.compare(path, index, column, found)?;
    }
    Ok(())
}
