use super::sheet::SheetState;
use crate::model::SourceRecord;
use anyhow::{Context, Result};
use rust_xlsxwriter::Workbook;

pub(super) fn write_headers(
    worksheet: &mut rust_xlsxwriter::Worksheet,
    headers: &[String],
) -> Result<()> {
    headers.iter().enumerate().try_for_each(|(column, header)| {
        let column = u16::try_from(column).context("header column conversion overflow")?;
        worksheet
            .write_string(0, column, header)
            .map(|_| ())
            .context("writing source header")
    })
}

pub(super) fn write_values(
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
