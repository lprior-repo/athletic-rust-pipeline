use super::super::super::read::Limits;
use super::stream;
use crate::report::{ReportError, ReportResult};
use std::io::Write;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn refused(result: ReportResult<usize>) -> TestResult {
    match result {
        Err(ReportError::Invariant { .. }) => Ok(()),
        other => Err(format!("expected bounded CSV refusal, found {other:?}").into()),
    }
}

fn limits(bytes: u64, records: usize) -> Limits {
    Limits { bytes, records }
}

#[test]
fn given_huge_quoted_field_with_newline_when_tiny_row_budget_then_no_data_row_is_visited(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("rows.csv");
    let field = "a\nb".repeat(32_768);
    std::fs::write(&path, format!("id,value\nx,\"{field}\"\n"))?;
    let mut visited = Vec::new();
    let result = stream::csv(&path, limits(131_072, 3), 16, 2, |index, _| {
        visited.push(index);
        Ok(())
    });
    refused(result)?;
    check!(eq; visited, vec![1]);
    Ok(())
}

#[test]
fn given_exact_decoded_limit_when_quoted_at_eof_then_fields_are_unchanged() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("rows.csv");
    std::fs::write(&path, b"id,value\r\nx,\"ab,c\n\"\"de\"")?;
    let mut found = Vec::new();
    let total = stream::csv(&path, limits(64, 2), 9, 2, |_, row| {
        let id = row
            .get(0)
            .ok_or_else(|| super::defect("missing id".into()))?;
        let value = row
            .get(1)
            .ok_or_else(|| super::defect("missing value".into()))?;
        found.push((id.to_owned(), value.to_owned()));
        Ok(())
    })?;
    check!(eq; total, 2);
    check!(eq; found, vec![("id".to_owned(), "value".to_owned()), ("x".to_owned(), "ab,c\n\"de".to_owned())]);
    Ok(())
}

#[test]
fn given_one_byte_over_limit_when_field_ends_at_eof_then_consumer_sees_nothing() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("rows.csv");
    std::fs::write(&path, b"\"123456789\"")?;
    let mut visited = 0;
    refused(stream::csv(&path, limits(32, 1), 8, 1, |_, _| {
        visited += 1;
        Ok(())
    }))?;
    check!(eq; visited, 0);
    Ok(())
}

#[test]
fn given_many_empty_columns_when_schema_has_two_fields_then_no_row_is_visited() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("rows.csv");
    std::fs::write(&path, ",".repeat(65_536))?;
    let mut visited = 0;
    refused(stream::csv(&path, limits(131_072, 1), 16, 2, |_, _| {
        visited += 1;
        Ok(())
    }))?;
    check!(eq; visited, 0);
    Ok(())
}

#[test]
fn given_invalid_utf8_or_cross_field_codepoint_when_decoded_then_no_row_is_visited() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("rows.csv");
    for bytes in [b"a,\xff\n".as_slice(), b"\xc3,\xa9\n".as_slice()] {
        std::fs::write(&path, bytes)?;
        let mut visited = 0;
        refused(stream::csv(&path, limits(32, 1), 16, 2, |_, _| {
            visited += 1;
            Ok(())
        }))?;
        check!(eq; visited, 0);
    }
    Ok(())
}

#[test]
fn given_record_limit_when_extra_row_is_read_then_only_budgeted_rows_are_visited() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("rows.csv");
    std::fs::write(&path, b"a\nb\nc\n")?;
    let mut visited = Vec::new();
    refused(stream::csv(&path, limits(32, 2), 1, 1, |index, _| {
        visited.push(index);
        Ok(())
    }))?;
    check!(eq; visited, vec![1, 2]);
    Ok(())
}

#[test]
fn given_file_at_cap_when_appended_during_read_then_growth_is_refused() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("rows.csv");
    std::fs::write(&path, b"a\nb\n")?;
    let mut visited = Vec::new();
    refused(stream::csv(&path, limits(4, 3), 1, 1, |index, _| {
        visited.push(index);
        if index == 1 {
            std::fs::OpenOptions::new()
                .append(true)
                .open(&path)
                .and_then(|mut file| file.write_all(b"c\n"))
                .map_err(|source| crate::report::io_error(&path, source))?;
        }
        Ok(())
    }))?;
    check!(eq; visited, vec![1, 2]);
    Ok(())
}

#[test]
fn given_csv_dialect_when_bom_blank_lines_crlf_and_open_quote_at_eof_then_records_match(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("rows.csv");
    std::fs::write(&path, b"\xef\xbb\xbfid,value\r\n\r\nx,\"kept\r\ntext")?;
    let mut found = Vec::new();
    let total = stream::csv(&path, limits(64, 2), 16, 2, |_, row| {
        found.push(
            row.get(1)
                .ok_or_else(|| super::defect("missing value".into()))?
                .to_owned(),
        );
        Ok(())
    })?;
    check!(eq; total, 2);
    check!(eq; found, vec!["value", "kept\r\ntext"]);
    Ok(())
}

#[test]
fn given_quoted_unicode_across_buffers_when_read_once_then_all_decoded_bytes_are_preserved(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("rows.csv");
    let prefix = "a".repeat(8190);
    let suffix = "z".repeat(4096);
    std::fs::write(&path, format!("\"{prefix}\"\"é,\r\n{suffix}\"\r\n"))?;
    let expected = format!("{prefix}\"é,\r\n{suffix}");
    let mut found = Vec::new();
    let total = stream::csv(&path, limits(16_384, 1), expected.len(), 1, |_, row| {
        found.push(
            row.get(0)
                .ok_or_else(|| super::defect("missing field".into()))?
                .to_owned(),
        );
        Ok(())
    })?;
    check!(eq; total, 1);
    check!(eq; found, vec![expected]);
    Ok(())
}

#[test]
fn given_wrong_or_excess_headers_when_table_is_read_then_consumer_is_not_called() -> TestResult {
    let directory = tempfile::tempdir()?;
    for bytes in [
        b"id,wrong\nx,y\n".as_slice(),
        b"id,value,\nx,y,\n".as_slice(),
    ] {
        std::fs::write(directory.path().join("rows.csv"), bytes)?;
        let mut visited = 0;
        let result = super::table(
            directory.path(),
            ("rows.csv", &["id", "value"]),
            1,
            &["id"],
            |_| {
                visited += 1;
                Ok(())
            },
        );
        refused(result.map(|_| 0))?;
        check!(eq; visited, 0);
    }
    Ok(())
}

#[test]
fn given_overflowing_or_unallocatable_limits_when_opened_then_consumer_is_not_called() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("rows.csv");
    std::fs::write(&path, b"a\n")?;
    for (bytes, row_bytes, columns) in [
        (u64::MAX, 8, 1),
        (32, usize::MAX, 1),
        (32, 8, usize::MAX),
        (32, 8, usize::MAX / 2),
    ] {
        let mut visited = 0;
        refused(stream::csv(
            &path,
            limits(bytes, 1),
            row_bytes,
            columns,
            |_, _| {
                visited += 1;
                Ok(())
            },
        ))?;
        check!(eq; visited, 0);
    }
    let mut visited = 0;
    refused(
        super::table(
            directory.path(),
            ("rows.csv", &["id"]),
            usize::MAX,
            &["id"],
            |_| {
                visited += 1;
                Ok(())
            },
        )
        .map(|_| 0),
    )?;
    check!(eq; visited, 0);
    Ok(())
}
