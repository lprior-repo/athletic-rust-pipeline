use super::digest::hash_file;
use super::headers::validate_extra_headers;
use super::sheet::verify_sheet;
use super::{SheetCounts, VerificationReport, GENERATED_SHEETS};
use crate::workbook_ingest;
use anyhow::{bail, Context, Result};
use calamine::{open_workbook, Reader, Xlsx};
use std::{fs::File, io::BufReader, path::Path};

/// Independently checks source-sheet order, headers, row keys, source fields, and source hash.
pub fn verify_fields(
    original: &Path,
    output: &Path,
    expected_sha: &crate::domain::identity::WorkbookDigest,
    extra_headers: &[String],
) -> Result<VerificationReport> {
    let source_before = hash_file(original)?;
    let expected = expected_sha.as_str().to_owned();
    if source_before != expected {
        bail!("original workbook hash before verification does not match expected digest");
    }
    validate_extra_headers(extra_headers)?;
    workbook_ingest::preflight(original)?;
    workbook_ingest::preflight(output)?;
    let (source_names, output_names) = open_sheet_names(original, output)?;
    if source_names != output_names {
        bail!("output workbook sheets do not match source sheets in original order");
    }
    let counts = count_sheets(original, output, &source_names, extra_headers)?;
    let source_after = hash_file(original)?;
    if source_after != expected {
        bail!("original workbook hash after verification does not match expected digest");
    }
    let sheet_count =
        u64::try_from(source_names.len()).context("sheet count conversion overflow")?;
    Ok(VerificationReport {
        expected_sha256: expected,
        source_sha256_before: source_before,
        source_sha256_after: source_after,
        source_hash_before_matches: true,
        source_hash_after_matches: true,
        source_sheet_count: sheet_count,
        output_sheet_count: sheet_count,
        matched_sheet_count: sheet_count,
        source_row_count: counts.source_rows,
        output_row_count: counts.output_rows,
        matched_row_count: counts.matched_rows,
        source_field_count: counts.source_fields,
        output_field_count: counts.output_fields,
        matched_field_count: counts.matched_fields,
        source_header_count: counts.source_headers,
        output_header_count: counts.output_headers,
        matched_header_count: counts.matched_headers,
        appended_header_count: u64::try_from(extra_headers.len())
            .context("extra header count conversion overflow")?,
    })
}

/// Verifies every source sheet against the output sheet and totals their counts.
fn count_sheets(
    original: &Path,
    output: &Path,
    source_names: &[String],
    extra_headers: &[String],
) -> Result<SheetCounts> {
    let mut source_book: Xlsx<BufReader<File>> =
        open_workbook(original).context("opening original workbook with calamine")?;
    let mut output_book: Xlsx<BufReader<File>> =
        open_workbook(output).context("opening output workbook with calamine")?;
    let mut source_header_bytes = 0_usize;
    let mut output_header_bytes = 0_usize;
    let counts = source_names.iter().try_fold(
        SheetCounts::default(),
        |mut total, name| -> Result<SheetCounts> {
            let sheet = verify_sheet(
                &mut source_book,
                &mut output_book,
                name,
                extra_headers,
                &mut source_header_bytes,
                &mut output_header_bytes,
            )?;
            accumulate_counts(&mut total, sheet)?;
            Ok(total)
        },
    )?;
    drop(source_book);
    drop(output_book);
    Ok(counts)
}

/// Adds one sheet's counts into the running totals.
fn accumulate_counts(total: &mut SheetCounts, sheet: SheetCounts) -> Result<()> {
    total.source_rows = total
        .source_rows
        .checked_add(sheet.source_rows)
        .context("source row count overflow")?;
    total.output_rows = total
        .output_rows
        .checked_add(sheet.output_rows)
        .context("output row count overflow")?;
    total.matched_rows = total
        .matched_rows
        .checked_add(sheet.matched_rows)
        .context("matched row count overflow")?;
    total.source_fields = total
        .source_fields
        .checked_add(sheet.source_fields)
        .context("source field count overflow")?;
    total.output_fields = total
        .output_fields
        .checked_add(sheet.output_fields)
        .context("output field count overflow")?;
    total.matched_fields = total
        .matched_fields
        .checked_add(sheet.matched_fields)
        .context("matched field count overflow")?;
    total.source_headers = total
        .source_headers
        .checked_add(sheet.source_headers)
        .context("source header count overflow")?;
    total.output_headers = total
        .output_headers
        .checked_add(sheet.output_headers)
        .context("output header count overflow")?;
    total.matched_headers = total
        .matched_headers
        .checked_add(sheet.matched_headers)
        .context("matched header count overflow")?;
    Ok(())
}

fn open_sheet_names(original: &Path, output: &Path) -> Result<(Vec<String>, Vec<String>)> {
    let source_book: Xlsx<BufReader<File>> =
        open_workbook(original).context("opening original workbook")?;
    let output_book: Xlsx<BufReader<File>> =
        open_workbook(output).context("opening output workbook")?;
    let source = source_book
        .sheet_names()
        .iter()
        .filter(|name| is_source_sheet(name))
        .cloned()
        .collect::<Vec<_>>();
    let output = output_book.sheet_names().to_owned();
    if source.is_empty() {
        bail!("original workbook contains no source worksheets");
    }
    Ok((source, output))
}

fn is_source_sheet(name: &str) -> bool {
    !GENERATED_SHEETS
        .iter()
        .any(|generated| name.eq_ignore_ascii_case(generated))
}
