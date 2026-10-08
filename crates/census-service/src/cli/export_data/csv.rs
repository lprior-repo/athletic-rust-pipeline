use anyhow::Result;
use census_store::read::{csv_failure, publish_atomically};
use census_store::{StoreError, StoreResult};
use std::borrow::Cow;
use std::path::Path;

pub fn write_csv(
    path: &Path,
    header: impl IntoIterator<Item = impl AsRef<[u8]>>,
    rows: &[Vec<String>],
) -> Result<()> {
    publish_atomically(path, |temporary| write_body(temporary, path, header, rows))?;
    Ok(())
}

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

fn write_body(
    temporary: &Path,
    published: &Path,
    header: impl IntoIterator<Item = impl AsRef<[u8]>>,
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
    fn csv_fields_are_safe_for_spreadsheets() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
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

        write_csv(&path, &["=header"], &rows)?;

        let bytes = std::fs::read(&path)?;
        let expected = b"=header\r\n'=cmd|' /C calc'!A0\r\n'+1+1\r\n'@SUM(1)\r\n-2.5\r\n'-dash-leading\r\nPlain Name\r\n";
        if bytes != expected {
            return Err(format!("spreadsheet-safe CSV: left={bytes:?}, right={expected:?}").into());
        }
        Ok(())
    }
}
