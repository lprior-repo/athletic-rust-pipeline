use std::collections::BTreeSet;
use std::path::Path;

use super::super::{cells::Cell, defect, read};
use crate::report::ReportResult;

mod decoder;
mod stream;

#[cfg(test)]
mod tests;

pub(super) struct Row<'a> {
    path: &'a Path,
    index: usize,
    header: &'a [&'a str],
    record: &'a decoder::Record<'a>,
}

impl Row<'_> {
    pub(super) fn get(&self, name: &str) -> ReportResult<&str> {
        let column = self.column(name)?;
        self.record.get(column).ok_or_else(|| {
            defect(format!(
                "{} record {} missing {name}",
                self.path.display(),
                self.index
            ))
        })
    }

    fn column(&self, name: &str) -> ReportResult<usize> {
        self.header
            .iter()
            .position(|column| *column == name)
            .ok_or_else(|| defect(format!("unknown normalized column {name}")))
    }

    pub(super) fn text(&self, name: &str, expected: impl Into<String>) -> ReportResult<()> {
        Cell::text(expected).compare(self.path, self.index, self.column(name)?, self.get(name)?)
    }

    pub(super) fn number(&self, name: &str, expected: impl ToString) -> ReportResult<()> {
        let value = self.get(name)?;
        let number = value
            .strip_prefix('\'')
            .map_or(value, core::convert::identity);
        number.parse::<i128>().map_err(|error| {
            defect(format!(
                "{} record {} {name} is not an integer: {error}",
                self.path.display(),
                self.index
            ))
        })?;
        self.text(name, expected.to_string())
    }

    pub(super) fn json(&self, name: &str, expected: &impl serde::Serialize) -> ReportResult<()> {
        let expected =
            serde_json::to_string(expected).map_err(|error| defect(error.to_string()))?;
        self.text(name, expected)
    }

    pub(super) fn literals(&self, values: &[(&str, &str)]) -> ReportResult<()> {
        values
            .iter()
            .try_for_each(|(name, value)| self.text(name, *value))
    }
}

pub(super) fn table(
    directory: &Path,
    schema: (&str, &[&str]),
    expected: usize,
    keys: &[&str],
    mut visit: impl FnMut(&Row<'_>) -> ReportResult<()>,
) -> ReportResult<u64> {
    let path = directory.join(schema.0);
    let mut seen = BTreeSet::new();
    let mut budget = crate::grouping::Admission::new("normalized readback keys");
    read_table(&path, schema.1, expected, |row| {
        admit_key(row, keys, &mut budget, &mut seen)?;
        visit(row)
    })
}

fn read_table(
    path: &Path,
    header: &[&str],
    expected: usize,
    mut visit: impl FnMut(&Row<'_>) -> ReportResult<()>,
) -> ReportResult<u64> {
    let limits = read::Limits {
        bytes: 268_435_456,
        records: decoder::add(expected, 2)?,
    };
    let total = stream::csv(
        path,
        limits,
        crate::grouping::ROW_BYTES,
        header.len(),
        |index, record| visit_record(path, header, index, record, &mut visit),
    )?;
    row_count(path, total, expected)?;
    let records = total
        .checked_sub(1)
        .ok_or_else(|| defect("normalized CSV header missing".into()))?;
    u64::try_from(records).map_err(|error| defect(format!("normalized CSV count: {error}")))
}

fn visit_record(
    path: &Path,
    header: &[&str],
    index: usize,
    record: &decoder::Record<'_>,
    visit: &mut impl FnMut(&Row<'_>) -> ReportResult<()>,
) -> ReportResult<()> {
    if index == 1 {
        return headers(path, record, header);
    }
    visit(&Row {
        path,
        index,
        header,
        record,
    })
}

fn row_count(path: &Path, total: usize, expected: usize) -> ReportResult<()> {
    if total == decoder::add(expected, 1)? {
        return Ok(());
    }
    Err(defect(format!(
        "{} holds {} rows; expected {expected} and one header",
        path.display(),
        total.saturating_sub(1)
    )))
}

fn headers(path: &Path, record: &decoder::Record<'_>, expected: &[&str]) -> ReportResult<()> {
    for (column, name) in expected.iter().enumerate() {
        let found = record
            .get(column)
            .ok_or_else(|| defect("missing normalized header".into()))?;
        if found != *name {
            return Err(defect(format!(
                "{} header column {column}: expected {name:?}, found {:?}",
                path.display(),
                super::super::excerpt(found)
            )));
        }
    }
    Ok(())
}

fn admit_key(
    row: &Row<'_>,
    keys: &[&str],
    budget: &mut crate::grouping::Admission,
    seen: &mut BTreeSet<Vec<String>>,
) -> ReportResult<()> {
    let key = keys
        .iter()
        .map(|name| row.get(name))
        .collect::<ReportResult<Vec<_>>>()?;
    budget.record(&key)?;
    if key.iter().any(|value| value.is_empty())
        || !seen.insert(
            key.iter()
                .map(|value| (*value).to_owned())
                .collect::<Vec<_>>(),
        )
    {
        return Err(defect(format!(
            "{} duplicate or empty key {key:?}",
            row.path.display()
        )));
    }
    Ok(())
}

pub(super) fn digest(value: &impl serde::Serialize) -> ReportResult<String> {
    census_domain::model::serialized_digest(value).map_err(|error| defect(error.to_string()))
}

pub(super) fn available(value: Option<&str>) -> &str {
    match value.filter(|value| !value.is_empty()) {
        Some(value) => value,
        None => "unavailable",
    }
}

pub(super) fn missing(kind: &str, id: &str) -> crate::report::ReportError {
    defect(format!(
        "normalized {kind} references missing or out-of-scope key {id}"
    ))
}
