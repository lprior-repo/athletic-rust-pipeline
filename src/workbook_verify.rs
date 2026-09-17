mod stream;

use crate::workbook_ingest;
use crate::workbook_ingest::stream::account_retained_headers;
use anyhow::{bail, Context, Result};
use calamine::{open_workbook, Reader, Xlsx};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::File,
    io::{self, BufReader, Write},
    path::Path,
};
use stream::{cell_event, CellRows, Row};

const MAX_EXCEL_ROW: u32 = 1_048_576;
const MAX_EXCEL_COLUMN: u32 = 16_384;
const GENERATED_SHEETS: [&str; 3] = ["Athletic Matches", "Corrections", "Summary"];

/// Aggregate evidence that an exported workbook preserves source fields.
/// This verifier does not prove any positive-match or decision rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VerificationReport {
    pub expected_sha256: String,
    pub source_sha256_before: String,
    pub source_sha256_after: String,
    pub source_hash_before_matches: bool,
    pub source_hash_after_matches: bool,
    pub source_sheet_count: u64,
    pub output_sheet_count: u64,
    pub matched_sheet_count: u64,
    pub source_row_count: u64,
    pub output_row_count: u64,
    pub matched_row_count: u64,
    pub source_field_count: u64,
    pub output_field_count: u64,
    pub matched_field_count: u64,
    pub source_header_count: u64,
    pub output_header_count: u64,
    pub matched_header_count: u64,
    pub appended_header_count: u64,
}

#[derive(Debug, Default)]
struct SheetCounts {
    source_rows: u64,
    output_rows: u64,
    matched_rows: u64,
    source_fields: u64,
    output_fields: u64,
    matched_fields: u64,
    source_headers: u64,
    output_headers: u64,
    matched_headers: u64,
}

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
            Ok(total)
        },
    )?;
    drop(source_book);
    drop(output_book);
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

fn validate_extra_headers(extra_headers: &[String]) -> Result<()> {
    if extra_headers.len()
        > usize::try_from(MAX_EXCEL_COLUMN).context("Excel column bound conversion overflow")?
    {
        bail!("extra header count exceeds Excel column limit");
    }
    let mut seen = BTreeSet::new();
    extra_headers.iter().try_for_each(|header| {
        if header.is_empty() || !seen.insert(header) {
            bail!("extra headers must be nonempty and unique");
        }
        Ok(())
    })
}

fn verify_sheet<RS, RO>(
    source_book: &mut Xlsx<RS>,
    output_book: &mut Xlsx<RO>,
    name: &str,
    extra_headers: &[String],
    source_header_bytes: &mut usize,
    output_header_bytes: &mut usize,
) -> Result<SheetCounts>
where
    RS: io::Read + io::Seek,
    RO: io::Read + io::Seek,
{
    let mut source_reader = source_book
        .worksheet_cells_reader(name)
        .context("reading source worksheet cells")?;
    let mut output_reader = output_book
        .worksheet_cells_reader(name)
        .context("reading output worksheet cells")?;
    let source_next = move || match source_reader.next_cell() {
        Ok(Some(cell)) => Ok(Some(cell_event(cell)?)),
        Ok(None) => Ok(None),
        Err(error) => Err(error).context("streaming source worksheet cells"),
    };
    let output_next = move || match output_reader.next_cell() {
        Ok(Some(cell)) => Ok(Some(cell_event(cell)?)),
        Ok(None) => Ok(None),
        Err(error) => Err(error).context("streaming output worksheet cells"),
    };
    let mut source_rows = CellRows::new(source_next);
    let mut output_rows = CellRows::new(output_next);
    let source_header = source_rows
        .next()
        .transpose()?
        .context("source worksheet has no nonempty header row")?;
    let output_header = output_rows
        .next()
        .transpose()?
        .context("output worksheet has no nonempty header row")?;
    let source_headers = parse_headers(&source_header, source_header_bytes)?;
    let output_headers = parse_headers(&output_header, output_header_bytes)?;
    if output_headers.len()
        != source_headers
            .len()
            .checked_add(extra_headers.len())
            .context("worksheet header width overflow")?
    {
        bail!("output worksheet header width differs from source plus supplied headers");
    }
    if output_headers.get(..source_headers.len()) != Some(source_headers.as_slice())
        || output_headers.get(source_headers.len()..) != Some(extra_headers)
    {
        bail!("output worksheet headers differ from source headers or supplied headers");
    }
    if extra_headers
        .iter()
        .any(|header| source_headers.iter().any(|source| source == header))
    {
        bail!("supplied extra header collides with a source header");
    }
    let mut counts = SheetCounts {
        source_headers: u64::try_from(source_headers.len())
            .context("source header count conversion overflow")?,
        output_headers: u64::try_from(output_headers.len())
            .context("output header count conversion overflow")?,
        matched_headers: u64::try_from(source_headers.len())
            .context("matched header count conversion overflow")?,
        ..SheetCounts::default()
    };
    loop {
        match (source_rows.next(), output_rows.next()) {
            (Some(source), Some(output)) => {
                let source = source?;
                let output = output?;
                compare_row(
                    &source,
                    &output,
                    source_headers.len(),
                    output_headers.len(),
                    &mut counts,
                )?;
            }
            (Some(source), None) => {
                source?;
                bail!("output worksheet is missing a source data row");
            }
            (None, Some(output)) => {
                output?;
                bail!("output worksheet contains an unexpected data row");
            }
            (None, None) => break,
        }
    }
    Ok(counts)
}
fn parse_headers(row: &Row, retained_header_bytes: &mut usize) -> Result<Vec<String>> {
    let header_bytes = row
        .cells
        .values()
        .filter(|value| !value.is_empty())
        .try_fold(0_usize, |total, value| {
            total
                .checked_add(value.len())
                .context("worksheet header byte count overflow")
        })?;
    account_retained_headers(retained_header_bytes, header_bytes)?;
    let last = row
        .cells
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .map(|(column, _)| *column)
        .max()
        .context("worksheet header is empty")?;
    let width = usize::try_from(
        last.checked_add(1)
            .context("worksheet header width overflow")?,
    )
    .context("worksheet header width conversion overflow")?;
    let headers = (0..width)
        .map(|column| {
            row.cells
                .get(&(column as u32))
                .map_or_else(String::new, Clone::clone)
        })
        .collect::<Vec<_>>();
    if headers.iter().any(String::is_empty) {
        bail!("worksheet header contains an empty field");
    }
    let mut seen = BTreeSet::new();
    if headers.iter().any(|header| !seen.insert(header)) {
        bail!("worksheet header contains duplicate fields");
    }
    Ok(headers)
}

fn compare_row(
    source: &Row,
    output: &Row,
    source_width: usize,
    output_width: usize,
    counts: &mut SheetCounts,
) -> Result<()> {
    if source.number > MAX_EXCEL_ROW
        || output.number > MAX_EXCEL_ROW
        || source.number != output.number
    {
        bail!("worksheet row keys differ or exceed Excel bounds");
    }
    if source.cells.iter().any(|(column, value)| {
        !value.is_empty() && usize::try_from(*column).map_or(true, |index| index >= source_width)
    }) {
        bail!("source worksheet contains an unexpected source field");
    }
    if output
        .cells
        .iter()
        .any(|(column, _)| usize::try_from(*column).map_or(true, |index| index >= output_width))
    {
        bail!("output worksheet contains a field outside its declared headers");
    }
    counts.source_rows = counts
        .source_rows
        .checked_add(1)
        .context("source row count overflow")?;
    counts.output_rows = counts
        .output_rows
        .checked_add(1)
        .context("output row count overflow")?;
    counts.matched_rows = counts
        .matched_rows
        .checked_add(1)
        .context("matched row count overflow")?;
    let width = u64::try_from(source_width).context("source width conversion overflow")?;
    counts.source_fields = counts
        .source_fields
        .checked_add(width)
        .context("source field count overflow")?;
    counts.output_fields = counts
        .output_fields
        .checked_add(width)
        .context("output field count overflow")?;
    counts.matched_fields = counts
        .matched_fields
        .checked_add(width)
        .context("matched field count overflow")?;
    (0..source_width).try_for_each(|column| {
        let expected = source
            .cells
            .get(&(column as u32))
            .map_or("", String::as_str);
        let actual = output
            .cells
            .get(&(column as u32))
            .map_or("", String::as_str);
        if expected == actual {
            Ok(())
        } else {
            bail!("source field differs at worksheet row and column")
        }
    })
}

fn hash_file(path: &Path) -> Result<String> {
    let mut input = BufReader::new(File::open(path).context("opening workbook for hashing")?);
    let mut sink = HashWriter {
        digest: Sha256::new(),
    };
    io::copy(&mut input, &mut sink).context("hashing workbook bytes")?;
    Ok(format!("{:x}", sink.digest.finalize()))
}

struct HashWriter {
    digest: Sha256,
}

impl Write for HashWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.digest.update(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
