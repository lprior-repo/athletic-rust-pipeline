use std::io::{BufWriter, Write};
use std::path::Path;

use super::super::read_table;
use crate::report::ReportError;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const FORMER_LIMIT: u64 = 512 * 1024 * 1024;
const ROWS: usize = 513;

#[test]
fn given_valid_csv_beyond_former_byte_limit_when_read_then_every_cell_matches() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("rows.csv");
    let payload = "v".repeat(
        crate::grouping::ROW_BYTES
            .checked_sub(32)
            .ok_or("normalized test row budget underflow")?,
    );
    let expected_bytes = write_csv(&path, &payload)?;
    check!(expected_bytes > FORMER_LIMIT);
    check!(expected_bytes < u64::try_from(crate::grouping::STAGE_BYTES)?);
    check!(eq; std::fs::metadata(&path)?.len(), expected_bytes);
    let mut visited = 0_usize;
    let records = read_table(&path, &["id", "value"], ROWS, |row| {
        row.text("id", format!("row-{visited}"))?;
        row.text("value", payload.as_str())?;
        visited = visited
            .checked_add(1)
            .ok_or_else(|| super::super::defect("normalized test row count overflow".into()))?;
        Ok(())
    })?;
    check!(eq; records, u64::try_from(ROWS)?);
    check!(eq; visited, ROWS);
    check!(eq; std::fs::metadata(&path)?.len(), expected_bytes);
    Ok(())
}

fn write_csv(path: &Path, payload: &str) -> TestResult<u64> {
    let mut writer = BufWriter::new(std::fs::File::create(path)?);
    let header = b"id,value\n";
    writer.write_all(header)?;
    let mut bytes = u64::try_from(header.len())?;
    for index in 0..ROWS {
        let prefix = format!("row-{index},");
        writer.write_all(prefix.as_bytes())?;
        writer.write_all(payload.as_bytes())?;
        writer.write_all(b"\n")?;
        let row_bytes = u64::try_from(prefix.len())?
            .checked_add(u64::try_from(payload.len())?)
            .and_then(|length| length.checked_add(1))
            .ok_or("normalized test row size overflow")?;
        bytes = bytes
            .checked_add(row_bytes)
            .ok_or("normalized test file size overflow")?;
    }
    writer.flush()?;
    Ok(bytes)
}

#[test]
fn given_sparse_csv_beyond_stage_limit_when_read_then_metadata_refuses_before_visit() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("rows.csv");
    let limit = u64::try_from(crate::grouping::STAGE_BYTES)?;
    let bytes = limit
        .checked_add(1)
        .ok_or("normalized test file limit overflow")?;
    std::fs::File::create(&path)?.set_len(bytes)?;
    let mut visited = 0_usize;
    let result = read_table(&path, &["id"], 0, |_| {
        visited = visited
            .checked_add(1)
            .ok_or_else(|| super::super::defect("normalized test row count overflow".into()))?;
        Ok(())
    });
    match result {
        Err(ReportError::Invariant { detail }) => check!(eq; detail, format!(
            "normalized CSV declares {bytes} bytes beyond the {limit} byte readback budget: {}",
            path.display()
        )),
        other => return Err(format!("expected declared-size refusal, found {other:?}").into()),
    }
    check!(eq; visited, 0);
    check!(eq; std::fs::metadata(&path)?.len(), bytes);
    Ok(())
}
