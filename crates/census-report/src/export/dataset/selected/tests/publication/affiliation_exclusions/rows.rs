use super::TestResult;
use calamine::{Reader, Xlsx};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub(super) type Row = BTreeMap<String, String>;

pub(super) fn csv(path: &Path) -> TestResult<Vec<Row>> {
    let contents = read(path)?;
    let mut reader = csv::Reader::from_reader(contents.as_slice());
    let headers = reader.headers()?.clone();
    check!(!headers.is_empty());
    check!(eq; headers.iter().collect::<BTreeSet<_>>().len(), headers.len());
    let mut rows = Vec::new();
    for record in reader.records() {
        check!(rows.len() < 128);
        let record = record?;
        check!(eq; record.len(), headers.len());
        rows.push(
            headers
                .iter()
                .zip(record.iter())
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect(),
        );
    }
    Ok(rows)
}

pub(super) fn keyed(
    rows: Vec<Row>,
    column: &str,
    expected: &BTreeSet<String>,
) -> TestResult<BTreeMap<String, Row>> {
    let mut found = BTreeMap::new();
    for row in rows {
        let key = row.get(column).ok_or("key column")?.clone();
        check!(found.insert(key, row).is_none());
    }
    check!(eq; found.keys().cloned().collect::<BTreeSet<_>>(), *expected);
    Ok(found)
}

pub(super) fn fields(row: &Row, expected: &[(&str, &str)]) -> TestResult {
    for (column, value) in expected {
        check!(eq; row.get(*column).map(String::as_str), Some(*value), "{column}");
    }
    Ok(())
}

pub(super) fn sheet(
    book: &mut Xlsx<std::io::BufReader<std::fs::File>>,
    name: &str,
) -> TestResult<Vec<Row>> {
    let range = book.worksheet_range(name)?;
    check!(range.height() <= 128);
    check!(range.width() <= 128);
    let mut rows = range.rows();
    let headers: Vec<_> = rows
        .next()
        .ok_or("worksheet header")?
        .iter()
        .map(ToString::to_string)
        .collect();
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

pub(super) fn json(path: &Path) -> TestResult<serde_json::Value> {
    Ok(serde_json::from_slice(&read(path)?)?)
}

pub(super) fn read(path: &Path) -> TestResult<Vec<u8>> {
    check!(std::fs::metadata(path)?.len() <= 16 * 1024 * 1024);
    Ok(std::fs::read(path)?)
}
