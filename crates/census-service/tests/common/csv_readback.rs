use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::{ensure, Context, Result};

pub(super) type Row = BTreeMap<String, String>;

pub(super) fn csv_rows(path: &Path) -> Result<Vec<Row>> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    ensure!(headers.iter().collect::<BTreeSet<_>>().len() == headers.len(), "duplicate CSV header");
    reader
        .records()
        .map(|record| {
            let record = record?;
            ensure!(record.len() == headers.len(), "malformed normalized row");
            Ok(headers
                .iter()
                .zip(record.iter())
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect())
        })
        .collect()
}

pub(super) fn field<'a>(row: &'a Row, name: &str) -> Result<&'a str> {
    row.get(name)
        .map(String::as_str)
        .with_context(|| format!("missing column {name}"))
}

#[test]
fn csv_readback_preserves_quoted_fields_and_duplicate_rows() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("rows.csv");
    std::fs::write(&path, "id,name\n1,\"two, lines\nquoted\"\n1,\"two, lines\nquoted\"\n")?;
    let expected = Row::from([("id".into(), "1".into()), ("name".into(), "two, lines\nquoted".into())]);
    ensure!(csv_rows(&path)? == vec![expected.clone(), expected.clone()]);
    ensure!(field(&expected, "name")? == "two, lines\nquoted");
    let error = field(&expected, "missing").err().context("missing field accepted")?;
    ensure!(error.to_string() == "missing column missing");
    Ok(())
}

#[test]
fn csv_readback_rejects_duplicate_headers_and_unequal_widths() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("rows.csv");
    std::fs::write(&path, "id,id\n1,2\n")?;
    let error = csv_rows(&path).err().context("duplicate header accepted")?;
    ensure!(error.to_string() == "duplicate CSV header");
    for (body, actual_len) in [("id,name\n1\n", 1), ("id,name\n1,2,3\n", 3)] {
        std::fs::write(&path, body)?;
        let error = csv_rows(&path).err().context("unequal width accepted")?;
        let error = error.downcast_ref::<csv::Error>().context("CSV decoder error")?;
        ensure!(matches!(error.kind(), csv::ErrorKind::UnequalLengths { expected_len: 2, len, .. } if *len == actual_len));
    }
    Ok(())
}
