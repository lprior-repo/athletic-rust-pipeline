use std::collections::HashMap;

use crate::{CrawlError, CrawlResult};

use super::ReadOutcome;

pub struct Header {
    columns: HashMap<String, usize>,
    width: usize,
}

impl Header {
    pub fn of(record: &csv::StringRecord) -> Self {
        let mut columns: HashMap<String, usize> = HashMap::with_capacity(record.len());
        for (index, name) in record.iter().enumerate() {
            let name = name
                .trim_start_matches('\u{feff}')
                .trim()
                .to_ascii_uppercase();
            columns.entry(name).or_insert(index);
        }
        Self {
            columns,
            width: record.len(),
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn require(&self, source: &str, names: &[&str]) -> CrawlResult<()> {
        let missing: Vec<&str> = names
            .iter()
            .copied()
            .filter(|name| !self.columns.contains_key(*name))
            .collect();
        if missing.is_empty() {
            return Ok(());
        }
        Err(CrawlError::Invariant {
            detail: format!(
                "the {source} file has no {} column; the reader maps by header name",
                missing.join(", ")
            ),
        })
    }

    pub fn index(&self, name: &str) -> Option<usize> {
        self.columns.get(name).copied()
    }
}

pub fn cell(record: &csv::StringRecord, index: Option<usize>) -> &str {
    index
        .and_then(|index| record.get(index))
        .map_or(Default::default(), core::convert::identity)
}

pub fn first<'a>(record: &'a csv::StringRecord, header: &Header, names: &[&str]) -> &'a str {
    for name in names {
        let value = cell(record, header.index(name));
        if !value.trim().is_empty() {
            return value;
        }
    }
    ""
}

pub fn refusal(error: csv::Error) -> CrawlError {
    CrawlError::Invariant {
        detail: format!("the file is not readable as a csv artifact: {error}"),
    }
}

pub fn read_rows<F>(
    text: &str,
    source: &str,
    required: &[&str],
    mut row: F,
) -> CrawlResult<ReadOutcome>
where
    F: FnMut(&Header, &csv::StringRecord, usize, &mut ReadOutcome),
{
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_reader(text.as_bytes());
    let mut records = reader.records();
    let header = match records.next() {
        Some(record) => Header::of(&record.map_err(refusal)?),
        None => {
            return Err(CrawlError::Invariant {
                detail: format!("the {source} artifact is empty"),
            })
        }
    };
    header.require(source, required)?;
    let mut outcome = ReadOutcome::new();
    for record in records {
        let record = record.map_err(refusal)?;
        let line = record
            .position()
            .map(|position| usize::try_from(position.line()).map_or(usize::MAX, |value| value))
            .map_or(Default::default(), core::convert::identity);
        if record.len() < header.width() {
            outcome.note(
                line,
                "row",
                format!(
                    "the row carries {} of {} cells: the columns it omits read as absent",
                    record.len(),
                    header.width()
                ),
            );
        }
        row(&header, &record, line, &mut outcome);
    }
    Ok(outcome)
}
