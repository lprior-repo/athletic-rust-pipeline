use super::*;
use calamine::{Reader, Xlsx};
use std::collections::{BTreeMap, BTreeSet};

pub(super) type Row = BTreeMap<String, String>;

pub(super) fn bytes(path: &Path) -> TestResult<Vec<u8>> {
    check!(std::fs::metadata(path)?.len() <= 16 * 1024 * 1024);
    Ok(std::fs::read(path)?)
}

pub(super) fn csv(path: &Path) -> TestResult<Vec<Row>> {
    let data = bytes(path)?;
    let mut reader = csv::Reader::from_reader(data.as_slice());
    let headers = reader.headers()?.clone();
    check!(!headers.is_empty());
    check!(eq; headers.iter().collect::<BTreeSet<_>>().len(), headers.len());
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record?;
        check!(eq; record.len(), headers.len());
        check!(rows.len() < 4096);
        rows.push(
            headers
                .iter()
                .zip(record.iter())
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
                .collect(),
        );
    }
    Ok(rows)
}

pub(super) fn unique<'a>(rows: &'a [Row], key: &str, value: &str) -> TestResult<&'a Row> {
    let mut matching = rows
        .iter()
        .filter(|row| row.get(key).map(String::as_str) == Some(value));
    let row = matching
        .next()
        .ok_or_else(|| format!("missing {key}={value}"))?;
    check!(matching.next().is_none(), "duplicate {key}={value}");
    Ok(row)
}

pub(super) fn fields(row: &Row, expected: &[(&str, &str)]) -> TestResult {
    for (key, value) in expected {
        check!(eq; row.get(*key).map(String::as_str), Some(*value), "field {key}");
    }
    Ok(())
}

pub(super) fn sheet(
    book: &mut Xlsx<std::io::BufReader<std::fs::File>>,
    name: &str,
) -> TestResult<Vec<Row>> {
    let range = book.worksheet_range(name)?;
    check!(range.height() <= 4096);
    check!(range.width() <= 128);
    let mut rows = range.rows();
    let headers = rows
        .next()
        .ok_or("workbook headers")?
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    check!(!headers.is_empty());
    check!(eq; headers.iter().collect::<BTreeSet<_>>().len(), headers.len());
    rows.map(|row| {
        check!(eq; row.len(), headers.len());
        Ok(headers
            .iter()
            .cloned()
            .zip(row.iter().map(ToString::to_string))
            .collect())
    })
    .collect()
}
