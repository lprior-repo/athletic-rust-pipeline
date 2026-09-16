use crate::{
    model::SourceRecord,
    runtime::{import::SourceManifest, snapshot},
};
use anyhow::{bail, Context, Result};
use rust_xlsxwriter::Workbook;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;
const MAX_EXCEL_ROW: u32 = 1_048_576;
const MAX_EXCEL_COLUMN: usize = 16_384;
#[derive(Debug, Clone)]
pub struct ExportRow {
    pub source: SourceRecord,
    pub extra_fields: std::collections::BTreeMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportSheetStats {
    pub name: String,
    pub rows: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportStats {
    pub sheets: Vec<ExportSheetStats>,
    pub aggregate_rows: u64,
    pub source_sha256: String,
}
pub struct WorkbookExport {
    workbook: Workbook,
    source: PathBuf,
    frozen: PathBuf,
    digest: crate::domain::identity::WorkbookDigest,
    extra_headers: Vec<String>,
    sheets: Vec<SheetState>,
    current_sheet: usize,
    expected_rows: u64,
    aggregate_rows: u64,
}
struct SheetState {
    name: String,
    source_headers: Vec<String>,
    headers: Vec<String>,
    expected_rows: u64,
    expected_last_row: u32,
    written_rows: u64,
    last_written_row: Option<u32>,
}
impl WorkbookExport {
    pub fn new(manifest: &SourceManifest, extra_headers: &[String]) -> Result<Self> {
        snapshot::verify(&manifest.original, &manifest.workbook)
            .context("verifying source workbook before export")?;
        let extra_headers = validate_extra_headers(extra_headers, &manifest.stats.sheets)?;
        let sheets = build_sheet_states(&manifest.stats.sheets, &extra_headers)?;
        let mut workbook = Workbook::new();
        sheets.iter().try_for_each(|sheet| {
            let worksheet = workbook.add_worksheet_with_constant_memory();
            worksheet
                .set_name(&sheet.name)
                .context("naming export worksheet")?;
            write_headers(worksheet, &sheet.headers)
        })?;
        Ok(Self {
            workbook,
            source: manifest.original.clone(),
            frozen: manifest.frozen.clone(),
            digest: manifest.workbook.clone(),
            extra_headers,
            sheets,
            current_sheet: 0,
            expected_rows: manifest.stats.actual_data_rows,
            aggregate_rows: 0,
        })
    }
    pub fn write(&mut self, row: ExportRow) -> Result<()> {
        let sheet_index = self
            .sheets
            .iter()
            .position(|sheet| sheet.name == row.source.sheet)
            .with_context(|| format!("unknown source worksheet {:?}", row.source.sheet))?;
        self.advance_to_sheet(sheet_index)?;
        let sheet = self
            .sheets
            .get(sheet_index)
            .context("export worksheet metadata disappeared")?;
        validate_row(&row, sheet, &self.extra_headers)?;
        let row_number = row.source.excel_row;
        let worksheet_row = row_number
            .checked_sub(1)
            .context("Excel row number underflow")?;
        write_values(
            &mut self.workbook,
            sheet_index,
            worksheet_row,
            sheet,
            &row.source,
            &row.extra_fields,
        )?;
        let sheet = self
            .sheets
            .get_mut(sheet_index)
            .context("export worksheet metadata disappeared")?;
        sheet.written_rows = sheet
            .written_rows
            .checked_add(1)
            .context("export worksheet row count overflow")?;
        sheet.last_written_row = Some(row_number);
        self.aggregate_rows = self
            .aggregate_rows
            .checked_add(1)
            .context("aggregate export row count overflow")?;
        Ok(())
    }
    pub fn finish(mut self, destination: &Path) -> Result<ExportStats> {
        self.sheets.iter().try_for_each(validate_sheet)?;
        if self.aggregate_rows != self.expected_rows {
            bail!("aggregate export row count differs from source manifest");
        }
        let parent = destination.parent().map_or_else(
            || Path::new("."),
            |path| {
                if path.as_os_str().is_empty() {
                    Path::new(".")
                } else {
                    path
                }
            },
        );
        let parent = fs::canonicalize(parent).context("resolving export destination directory")?;
        reject_destination(destination, &parent, &self.source, &self.frozen)?;
        let temporary =
            NamedTempFile::new_in(&parent).context("creating atomic export temporary")?;
        self.workbook
            .save(temporary.path())
            .context("writing XLSX export")?;
        let mut temporary = temporary;
        temporary
            .as_file_mut()
            .flush()
            .context("flushing XLSX export")?;
        temporary
            .as_file()
            .sync_all()
            .context("syncing XLSX export")?;
        snapshot::verify(&self.source, &self.digest)
            .context("verifying source workbook before export publication")?;
        match temporary.persist_noclobber(destination) {
            Ok(_) => {}
            Err(error) => {
                return Err(error.error).context("publishing XLSX export without clobbering")
            }
        }
        File::open(&parent)
            .context("opening export directory")?
            .sync_all()
            .context("syncing export directory")?;
        Ok(ExportStats {
            sheets: self
                .sheets
                .into_iter()
                .map(|sheet| ExportSheetStats {
                    name: sheet.name,
                    rows: sheet.written_rows,
                })
                .collect(),
            aggregate_rows: self.aggregate_rows,
            source_sha256: self.digest.as_str().to_owned(),
        })
    }
    fn advance_to_sheet(&mut self, sheet_index: usize) -> Result<()> {
        if sheet_index < self.current_sheet {
            bail!("source worksheets must be written in manifest order");
        }
        (self.current_sheet..sheet_index).try_for_each(|index| {
            self.sheets
                .get(index)
                .context("export worksheet metadata disappeared")
                .and_then(validate_sheet)
        })?;
        self.current_sheet = sheet_index;
        Ok(())
    }
}
fn validate_extra_headers(
    extra_headers: &[String],
    sheets: &[crate::model::SheetStats],
) -> Result<Vec<String>> {
    if extra_headers.len() > MAX_EXCEL_COLUMN {
        bail!("extra header count exceeds Excel column limit");
    }
    let mut seen = BTreeSet::new();
    extra_headers.iter().try_for_each(|header| {
        if header.is_empty() || !seen.insert(header) {
            bail!("extra headers must be nonempty and unique");
        }
        if sheets
            .iter()
            .any(|sheet| sheet.headers.iter().any(|source| source == header))
        {
            bail!("extra header collides with a source header: {header:?}");
        }
        Ok(())
    })?;
    Ok(extra_headers.to_owned())
}
fn build_sheet_states(
    stats: &[crate::model::SheetStats],
    extra_headers: &[String],
) -> Result<Vec<SheetState>> {
    if stats.is_empty() {
        bail!("source manifest contains no worksheets");
    }
    let mut names = BTreeSet::new();
    stats
        .iter()
        .map(|sheet| {
            if !names.insert(sheet.name.clone()) {
                bail!("duplicate source worksheet {:?}", sheet.name);
            }
            validate_source_headers(sheet)?;
            if sheet
                .headers
                .len()
                .checked_add(extra_headers.len())
                .is_none_or(|width| width > MAX_EXCEL_COLUMN)
            {
                bail!("worksheet {:?} exceeds Excel column limit", sheet.name);
            }
            let mut headers = sheet.headers.clone();
            headers.extend(extra_headers.iter().cloned());
            Ok(SheetState {
                name: sheet.name.clone(),
                source_headers: sheet.headers.clone(),
                headers,
                expected_rows: sheet.actual_data_rows,
                expected_last_row: sheet.last_actual_row,
                written_rows: 0,
                last_written_row: None,
            })
        })
        .collect()
}
fn validate_source_headers(sheet: &crate::model::SheetStats) -> Result<()> {
    let mut headers = BTreeSet::new();
    sheet.headers.iter().try_for_each(|header| {
        if header.is_empty() || !headers.insert(header) {
            bail!("worksheet {:?} has empty or duplicate headers", sheet.name);
        }
        Ok(())
    })
}
fn write_headers(worksheet: &mut rust_xlsxwriter::Worksheet, headers: &[String]) -> Result<()> {
    headers.iter().enumerate().try_for_each(|(column, header)| {
        let column = u16::try_from(column).context("header column conversion overflow")?;
        worksheet
            .write_string(0, column, header)
            .map(|_| ())
            .context("writing source header")
    })
}
fn validate_row(row: &ExportRow, sheet: &SheetState, extra_headers: &[String]) -> Result<()> {
    let key = crate::domain::identity::SourceRowKey::parse(&row.source.source_key)
        .context("parsing source row key")?;
    if key.sheet() != sheet.name
        || key.row() != row.source.excel_row
        || row.source.sheet != sheet.name
    {
        bail!("source row metadata does not match worksheet and source key");
    }
    if row.source.excel_row > MAX_EXCEL_ROW {
        bail!("source row exceeds Excel row limit");
    }
    if sheet
        .last_written_row
        .is_some_and(|last| row.source.excel_row <= last)
    {
        bail!("source rows must be strictly increasing within each worksheet");
    }
    if row
        .source
        .fields
        .keys()
        .any(|field| !sheet.source_headers.iter().any(|header| header == field))
    {
        bail!("source row contains an undeclared source field");
    }
    if row
        .extra_fields
        .keys()
        .any(|field| !extra_headers.iter().any(|header| header == field))
    {
        bail!("source row contains an undeclared extra field");
    }
    if sheet.written_rows >= sheet.expected_rows {
        bail!("source worksheet received more rows than its manifest cardinality");
    }
    Ok(())
}
fn write_values(
    workbook: &mut Workbook,
    sheet_index: usize,
    worksheet_row: u32,
    sheet: &SheetState,
    source: &SourceRecord,
    extra_fields: &std::collections::BTreeMap<String, String>,
) -> Result<()> {
    let worksheet = workbook
        .worksheet_from_index(sheet_index)
        .context("selecting export worksheet")?;
    sheet
        .headers
        .iter()
        .enumerate()
        .try_for_each(|(column, header)| {
            let value = source
                .fields
                .get(header)
                .or_else(|| extra_fields.get(header))
                .map_or("", String::as_str);
            let column = u16::try_from(column).context("data column conversion overflow")?;
            worksheet
                .write_string(worksheet_row, column, value)
                .map(|_| ())
                .context("writing source data")
        })
}
fn validate_sheet(sheet: &SheetState) -> Result<()> {
    if sheet.written_rows != sheet.expected_rows {
        bail!(
            "worksheet {:?} row cardinality differs from source manifest",
            sheet.name
        );
    }
    if sheet.last_written_row.map_or(0, std::convert::identity) != sheet.expected_last_row {
        bail!(
            "worksheet {:?} last row differs from source manifest",
            sheet.name
        );
    }
    Ok(())
}
fn reject_destination(
    destination: &Path,
    parent: &Path,
    source: &Path,
    frozen: &Path,
) -> Result<()> {
    match fs::symlink_metadata(destination) {
        Ok(_) => bail!("export destination already exists or is a symlink"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("checking export destination"),
    }
    let name = destination
        .file_name()
        .context("export destination has no filename")?;
    let candidate = parent.join(name);
    [source, frozen].iter().try_for_each(|path| {
        if fs::canonicalize(path).is_ok_and(|canonical| canonical == candidate) {
            bail!("export destination must not replace the source workbook");
        }
        Ok(())
    })
}
