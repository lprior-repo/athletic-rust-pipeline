use super::{error, index::Index, ComparisonCounts};
use crate::report::ReportResult;
use crate::workbook::verify::{
    canonical::Value,
    xlsx::{Archive, Cells, Profile},
};
use census_domain::model::{person_key, SourceIdentity, SourceNamespace};
use std::path::Path;

const HEADERS: [&str; 9] = [
    "AthleteID",
    "Names",
    "GradeID",
    "Teams",
    "States",
    "Events",
    "Result Count",
    "Other Observed Grades",
    "Identity Match Status",
];
const SHEET: &str = "All Athletes";
const MAX_TEXT: usize = 16 * 1024;

pub(super) fn compare(
    path: &Path,
    expected: usize,
    index: &mut Index,
    year: i16,
) -> ReportResult<ComparisonCounts> {
    let archive = Archive::open(path, Profile::inline())?;
    if archive.uses_shared_strings() || archive.sheet_names()? != [SHEET] {
        return Err(error(
            "reference profile requires exactly All Athletes and no shared strings",
        ));
    }
    let rows = expected
        .checked_add(1)
        .ok_or_else(|| error("reference row count overflow"))?;
    let mut visitor = ReferenceRows::new(index, year);
    let actual = archive.read_sheet(SHEET, rows, HEADERS.len(), &mut visitor)?;
    visitor.completed?;
    if actual != Some(rows) || visitor.counts.reference_rows != expected {
        return Err(error("reference traversal does not match bound row count"));
    }
    Ok(visitor.counts)
}

struct ReferenceRows<'a> {
    index: &'a mut Index,
    year: i16,
    current: Option<usize>,
    next: usize,
    values: [Value; 9],
    counts: ComparisonCounts,
    completed: ReportResult<()>,
}

impl<'a> ReferenceRows<'a> {
    fn new(index: &'a mut Index, year: i16) -> Self {
        Self {
            index,
            year,
            current: None,
            next: 0,
            values: std::array::from_fn(|_| Value::Empty),
            counts: ComparisonCounts::default(),
            completed: Ok(()),
        }
    }

    fn finish_row(&mut self) -> ReportResult<()> {
        let Some(row) = self.current.take() else {
            return Ok(());
        };
        if row != self.next {
            return Err(error("reference contains a missing row"));
        }
        if row == 0 {
            for (value, header) in self.values.iter().zip(HEADERS) {
                if !matches!(value, Value::Text(text) if text == header) {
                    return Err(error("reference headers differ from the bound profile"));
                }
            }
        } else {
            let id = id(self
                .values
                .first()
                .ok_or_else(|| error("reference ID column missing"))?)?;
            let name = text(self.values.get(1))?;
            let grade = self
                .values
                .get(2)
                .ok_or_else(|| error("reference grade column missing"))?;
            if !matches!(grade, Value::Number(number) if *number == 11.0)
                && !matches!(grade, Value::Text(text) if text == "11")
            {
                self.counts.required_fields_unassessed += 1;
            }
            self.index.observe(id, name, self.year, &mut self.counts)?;
        }
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(|| error("reference row counter overflow"))?;
        self.values = std::array::from_fn(|_| Value::Empty);
        Ok(())
    }
}

impl Cells for ReferenceRows<'_> {
    fn cell(&mut self, row: usize, column: usize, value: Value) -> ReportResult<()> {
        if self.current != Some(row) {
            self.finish_row()?;
            self.current = Some(row);
        }
        if matches!(&value, Value::Text(text) if text.len() > MAX_TEXT) {
            return Err(error("reference cell exceeds comparison text budget"));
        }
        let slot = self
            .values
            .get_mut(column)
            .ok_or_else(|| error("reference column exceeds profile"))?;
        *slot = value;
        Ok(())
    }

    fn finish(&mut self, _rows: usize) {
        self.completed = self.finish_row();
    }
}

fn text(value: Option<&Value>) -> ReportResult<&str> {
    match value {
        Some(Value::Text(text)) => Ok(text),
        Some(Value::Empty) => Ok(""),
        _ => Err(error("reference name must be text")),
    }
}

pub(super) fn id(value: &Value) -> ReportResult<u64> {
    let value = match value {
        Value::Text(text) => text.clone(),
        Value::Number(number)
            if number.is_finite()
                && *number > 0.0
                && *number <= 9_007_199_254_740_991.0
                && number.fract() == 0.0 =>
        {
            number.to_string()
        }
        _ => {
            return Err(error(
                "reference athlete ID is not a lossless positive integer",
            ))
        }
    };
    let identity = SourceIdentity::new(SourceNamespace::athletic_net("athlete"), value);
    person_key(&identity)
        .map(|(_, id)| id)
        .ok_or_else(|| error("reference athlete ID is not canonical"))
}
