use super::{
    destination::reject_destination,
    plan::{build_sheet_states, validate_extra_headers},
    sheet::SheetState,
    types::{ExportRow, ExportSheetStats, ExportStats},
    validate::{validate_row, validate_sheet},
    write::{write_headers, write_values},
};
use crate::runtime::{import::SourceManifest, snapshot};
use anyhow::{bail, Context, Result};
use rust_xlsxwriter::Workbook;
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::NamedTempFile;

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
