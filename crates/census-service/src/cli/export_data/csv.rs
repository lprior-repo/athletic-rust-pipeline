//! CSV writing utility.
//!
//! Excel-class applications evaluate fields starting with `=`, `+`, `-`, or `@` as formulas, so a
//! leading apostrophe is the standard control for the third-party text in these data products.

use anyhow::Result;
use census_store::read::{csv_failure, publish_atomically};
use census_store::{StoreError, StoreResult};
use std::borrow::Cow;
use std::path::Path;

/// Write a CSV file with the given header and rows.
///
/// Published by rename like every other artifact this repository emits: the export is read by the
/// census document, which opens these tables expecting a complete one, and an in-place write that a
/// crash interrupted leaves a short file the next document pass takes for the real thing.
pub fn write_csv(path: &Path, header: &[&str], rows: &[Vec<String>]) -> Result<()> {
    publish_atomically(path, |temporary| write_body(temporary, path, header, rows))?;
    Ok(())
}

/// Escape third-party text only when a spreadsheet could interpret its first character.
fn escape_field(field: &str) -> Cow<'_, str> {
    let needs_escape = match field.as_bytes().first().copied() {
        Some(b'=' | b'+' | b'@' | b'\t' | b'\r') => true,
        Some(b'-') => field.parse::<f64>().is_err(),
        _ => false,
    };
    if needs_escape {
        Cow::Owned(format!("'{field}"))
    } else {
        Cow::Borrowed(field)
    }
}

/// Encode the table into `temporary`, the file [`publish_atomically`] renames to `published`.
fn write_body(
    temporary: &Path,
    published: &Path,
    header: &[&str],
    rows: &[Vec<String>],
) -> StoreResult<()> {
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_path(temporary)
        .map_err(|error| csv_failure(published, error))?;
    writer
        .write_record(header)
        .map_err(|error| csv_failure(published, error))?;
    for row in rows {
        let escaped: Vec<Cow<'_, str>> = row.iter().map(|field| escape_field(field)).collect();
        writer
            .write_record(escaped.iter().map(|field| &**field))
            .map_err(|error| csv_failure(published, error))?;
    }
    writer.flush().map_err(|source| StoreError::Io {
        path: published.to_path_buf(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::write_csv;

    #[test]
    fn csv_fields_are_safe_for_spreadsheets() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("export.csv");
        let rows = [
            "=cmd|' /C calc'!A0",
            "+1+1",
            "@SUM(1)",
            "-2.5",
            "-dash-leading",
            "Plain Name",
        ]
        .into_iter()
        .map(str::to_owned)
        .map(|value| vec![value])
        .collect::<Vec<_>>();

        write_csv(&path, &["=header"], &rows).expect("CSV publishes");

        let bytes = std::fs::read(&path).expect("published CSV reads");
        assert_eq!(
            bytes,
            b"=header\r\n'=cmd|' /C calc'!A0\r\n'+1+1\r\n'@SUM(1)\r\n-2.5\r\n'-dash-leading\r\nPlain Name\r\n"
        );
    }
}
