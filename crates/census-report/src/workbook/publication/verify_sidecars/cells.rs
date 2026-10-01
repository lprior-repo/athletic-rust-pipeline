use crate::report::{ReportError, ReportResult};
use std::path::Path;

use super::{defect, excerpt};

pub(super) enum Cell {
    Text(Option<String>),
    Integer(Option<String>),
    Decimal(Option<f64>),
}

impl Cell {
    pub(super) fn text(value: impl Into<String>) -> Self {
        Self::Text(Some(value.into()))
    }

    pub(super) fn optional(value: Option<impl Into<String>>) -> Self {
        Self::Text(value.map(Into::into))
    }

    pub(super) fn integer(value: impl Into<String>) -> Self {
        Self::Integer(Some(value.into()))
    }

    pub(super) fn optional_integer(value: Option<impl Into<String>>) -> Self {
        Self::Integer(value.map(Into::into))
    }

    pub(super) fn decimal(value: Option<f64>) -> Self {
        Self::Decimal(value)
    }

    pub(super) fn compare(
        &self,
        path: &Path,
        record: usize,
        column: usize,
        found: &str,
    ) -> ReportResult<()> {
        match self {
            Self::Text(Some(value)) => compare_text(path, record, column, value, found),
            Self::Text(None) => compare_empty(path, record, column, found),
            Self::Integer(Some(value)) => compare_exact(path, record, column, value, found),
            Self::Integer(None) => compare_empty(path, record, column, found),
            Self::Decimal(None) => compare_empty(path, record, column, found),
            Self::Decimal(Some(value)) => match found.parse::<f64>() {
                Ok(parsed) if parsed == *value => Ok(()),
                _ => Err(mismatch(path, record, column, &value.to_string(), found)),
            },
        }
    }
}

fn compare_text(
    path: &Path,
    record: usize,
    column: usize,
    value: &str,
    found: &str,
) -> ReportResult<()> {
    let expected = crate::csv_safety::protect_owned(value.to_string()).map_err(|source| {
        defect(format!(
            "protecting CSV text for {}: {source}",
            path.display()
        ))
    })?;
    if expected == found {
        Ok(())
    } else {
        Err(mismatch(path, record, column, &expected, found))
    }
}

fn compare_exact(
    path: &Path,
    record: usize,
    column: usize,
    value: &str,
    found: &str,
) -> ReportResult<()> {
    if value == found {
        Ok(())
    } else {
        Err(mismatch(path, record, column, value, found))
    }
}

fn compare_empty(path: &Path, record: usize, column: usize, found: &str) -> ReportResult<()> {
    if found.is_empty() {
        Ok(())
    } else {
        Err(mismatch(path, record, column, "", found))
    }
}

fn mismatch(path: &Path, record: usize, column: usize, expected: &str, found: &str) -> ReportError {
    defect(format!(
        "{} record {record} column {column}: expected {:?}, found {:?}",
        path.display(),
        excerpt(expected),
        excerpt(found)
    ))
}
