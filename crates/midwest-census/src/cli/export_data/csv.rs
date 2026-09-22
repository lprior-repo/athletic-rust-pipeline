//! CSV writing utility.

use anyhow::{Context, Result};
use std::path::Path;

/// Write a CSV file with the given header and rows.
pub fn write_csv(path: &Path, header: &[&str], rows: &[Vec<String>]) -> Result<()> {
    let p = path.display();
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_path(path)
        .with_context(|| format!("creating csv at {p}"))?;
    writer.write_record(header)?;
    for row in rows {
        writer.write_record(row)?;
    }
    writer.flush()?;
    Ok(())
}
