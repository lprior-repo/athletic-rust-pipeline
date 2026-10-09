use std::collections::HashMap;

use crate::{CrawlError, CrawlResult};

use super::{budget::ReadBudget, ReadOutcome};
use census_domain::school_directory::DirectoryError;
#[path = "artifact/decoded.rs"]
mod decoded;
pub use decoded::CsvRow;
use decoded::Decoder;

pub struct Header {
    columns: HashMap<String, usize>,
    width: usize,
}

impl Header {
    pub fn of(record: &CsvRow<'_>) -> CrawlResult<Self> {
        let mut columns = HashMap::new();
        columns
            .try_reserve(record.len())
            .map_err(|_| DirectoryError::Allocation {
                resource: "directory CSV header",
            })?;
        record
            .iter()
            .enumerate()
            .try_for_each(|(index, name)| -> Result<(), DirectoryError> {
                let name = name.trim_start_matches('\u{feff}').trim();
                super::limits::check("directory CSV header name bytes", name.len(), 128)?;
                let mut name = super::limits::text(name)?;
                name.make_ascii_uppercase();
                columns.entry(name).or_insert(index);
                Ok(())
            })?;
        Ok(Self {
            columns,
            width: record.len(),
        })
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn require(&self, source: &str, names: &[&str]) -> CrawlResult<()> {
        let missing = names.iter().find(|name| !self.columns.contains_key(**name));
        let Some(missing) = missing else {
            return Ok(());
        };
        Err(CrawlError::Invariant {
            detail: super::issue_detail(format_args!("the {source} file has no {missing} column"))?,
        })
    }

    pub fn index(&self, name: &str) -> Option<usize> {
        self.columns.get(name).copied()
    }
}

pub fn cell<'a>(record: &'a CsvRow<'_>, index: Option<usize>) -> &'a str {
    index
        .and_then(|index| record.get(index))
        .map_or(Default::default(), core::convert::identity)
}

pub fn first<'a>(record: &'a CsvRow<'_>, header: &Header, names: &[&str]) -> &'a str {
    names
        .iter()
        .map(|name| cell(record, header.index(name)))
        .find(|value| !value.trim().is_empty())
        .map_or("", core::convert::identity)
}

pub fn refusal(error: csv::Error) -> CrawlError {
    match super::issue_detail(format_args!(
        "the file is not readable as a CSV artifact: {error}"
    )) {
        Ok(detail) => CrawlError::Invariant { detail },
        Err(error) => error.into(),
    }
}

pub fn read_rows<F>(text: &str, source: &str, required: &[&str], row: F) -> CrawlResult<ReadOutcome>
where
    F: FnMut(&Header, &CsvRow<'_>, usize, &mut ReadOutcome) -> Result<(), DirectoryError>,
{
    read_rows_with_budget(text, source, required, row, ReadBudget::Page)
}

pub(crate) fn read_national_rows<F>(
    text: &str,
    source: &str,
    required: &[&str],
    row: F,
) -> CrawlResult<ReadOutcome>
where
    F: FnMut(&Header, &CsvRow<'_>, usize, &mut ReadOutcome) -> Result<(), DirectoryError>,
{
    read_rows_with_budget(text, source, required, row, ReadBudget::NationalFile)
}

fn read_rows_with_budget<F>(
    text: &str,
    source: &str,
    required: &[&str],
    mut row: F,
    budget: ReadBudget,
) -> CrawlResult<ReadOutcome>
where
    F: FnMut(&Header, &CsvRow<'_>, usize, &mut ReadOutcome) -> Result<(), DirectoryError>,
{
    let mut decoder = Decoder::new(text, budget.input_bytes())?;
    let header_row = decoder.next()?.ok_or_else(|| CrawlError::Invariant {
        detail: "directory artifact has no header".into(),
    })?;
    let header = Header::of(&header_row)?;
    header.require(source, required)?;
    let mut outcome = ReadOutcome::with_budget(budget);
    let stop = (0..=budget.source_rows())
        .find_map(|ordinal| visit(&mut decoder, &header, &mut row, &mut outcome, ordinal))
        .ok_or_else(|| CrawlError::Invariant {
            detail: "directory CSV frontier was not classified".into(),
        })?;
    match stop {
        Stop::End(0) => outcome.stop(
            2,
            DirectoryError::Representation {
                detail: "directory CSV has no data rows".into(),
            },
        ),
        Stop::End(_) => outcome.finish(),
        Stop::Refused(line, error) => outcome.stop(line, error),
    }
    Ok(outcome)
}

enum Stop {
    End(usize),
    Refused(usize, DirectoryError),
}

fn visit<F>(
    decoder: &mut Decoder<'_>,
    header: &Header,
    row: &mut F,
    outcome: &mut ReadOutcome,
    ordinal: usize,
) -> Option<Stop>
where
    F: FnMut(&Header, &CsvRow<'_>, usize, &mut ReadOutcome) -> Result<(), DirectoryError>,
{
    let record = match decoder.next() {
        Ok(Some(record)) => record,
        Ok(None) => return Some(Stop::End(ordinal)),
        Err(error) => return Some(Stop::Refused(decoder.row_start(), error)),
    };
    if ordinal == outcome.source_row_limit() {
        return Some(Stop::Refused(
            record.line,
            DirectoryError::Capacity {
                resource: "directory CSV source rows",
                requested: ordinal + 1,
                limit: outcome.source_row_limit(),
            },
        ));
    }
    let result = note_short_row(outcome, &record, header.width())
        .and_then(|()| row(header, &record, record.line, outcome));
    result.err().map(|error| Stop::Refused(record.line, error))
}

fn note_short_row(
    outcome: &mut ReadOutcome,
    record: &CsvRow<'_>,
    width: usize,
) -> Result<(), DirectoryError> {
    if record.len() >= width {
        return Ok(());
    }
    let detail = super::issue_detail(format_args!(
        "the row carries {} of {width} cells: the columns it omits read as absent",
        record.len()
    ))?;
    outcome.note(record.line, "row", detail)
}
