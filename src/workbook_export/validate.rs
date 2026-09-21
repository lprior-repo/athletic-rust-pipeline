use super::sheet::{SheetState, MAX_EXCEL_ROW};
use super::types::ExportRow;
use anyhow::{bail, Context, Result};

pub(super) fn validate_row(
    row: &ExportRow,
    sheet: &SheetState,
    extra_headers: &[String],
) -> Result<()> {
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

pub(super) fn validate_sheet(sheet: &SheetState) -> Result<()> {
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
