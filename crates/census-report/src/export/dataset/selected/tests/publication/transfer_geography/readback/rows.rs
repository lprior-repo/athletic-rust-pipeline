use super::{Path, TestResult};
use calamine::{Reader, Xlsx};
use std::collections::{BTreeMap, BTreeSet};

type Row = BTreeMap<String, String>;

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
        .ok_or("workbook headers")?
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

pub(super) fn unique<'a>(rows: &'a [Row], key: &str, value: &str) -> TestResult<&'a Row> {
    let mut matches = rows
        .iter()
        .filter(|row| row.get(key).map(String::as_str) == Some(value));
    let row = matches
        .next()
        .ok_or_else(|| format!("missing {key}={value}"))?;
    check!(matches.next().is_none());
    Ok(row)
}

pub(super) fn fields(row: &Row, expected: &[(&str, &str)]) -> TestResult {
    for (name, value) in expected {
        check!(eq; row.get(*name).map(String::as_str), Some(*value), "{name}");
    }
    Ok(())
}

pub(super) fn csv_rows(path: &Path) -> TestResult<Vec<Row>> {
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
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
                .collect(),
        );
    }
    Ok(rows)
}

pub(super) fn read(path: &Path) -> TestResult<Vec<u8>> {
    check!(std::fs::metadata(path)?.len() <= 16 * 1024 * 1024);
    Ok(std::fs::read(path)?)
}
