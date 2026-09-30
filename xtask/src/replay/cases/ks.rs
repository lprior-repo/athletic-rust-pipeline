use crate::replay::{ensure_rows, unmapped, Capture};
use anyhow::Result;
use census_crawl::ks;

pub(super) fn ks_directory(capture: &Capture<'_>) -> Result<String> {
    let (file, body) = (capture.file, capture.body);
    if file.starts_with("kshsaa_directory_") {
        let records = ks::parse_records(body)?;
        ensure_rows(file, records.len(), "school records")?;
        return Ok(format!("directory records={}", records.len()));
    }
    unmapped("ks", file)
}
