use super::preflight::WorksheetPreflight;
use crate::xlsx::cells;
use anyhow::{bail, Context, Result};
use quick_xml::events::Event;

#[derive(Debug, Default)]
pub(super) struct WorksheetGuard {
    in_sheet_data: bool,
    saw_sheet_data: bool,
    row_open: bool,
    cell_open: bool,
    row_number: Option<u32>,
    last_column: Option<usize>,
    last_row: Option<u32>,
    rows: u64,
    declared_dimension: Option<String>,
}

impl WorksheetGuard {
    pub(super) fn observe(&mut self, event: &Event<'_>) -> Result<()> {
        match event {
            Event::Start(element) => self.start(element),
            Event::Empty(element) => self.empty(element),
            Event::End(element) => self.end(element.local_name().as_ref()),
            _ => Ok(()),
        }
    }

    fn start(&mut self, element: &quick_xml::events::BytesStart<'_>) -> Result<()> {
        match element.local_name().as_ref() {
            b"sheetData" => {
                if self.saw_sheet_data {
                    bail!("worksheet contains duplicate sheetData elements");
                }
                self.in_sheet_data = true;
                self.saw_sheet_data = true;
                Ok(())
            }
            b"row" if self.in_sheet_data => self.start_row(element),
            b"c" if self.row_open => self.start_cell(element),
            b"c" => bail!("worksheet cell is outside a row"),
            b"dimension" => self.dimension(element),
            _ => Ok(()),
        }
    }
    fn empty(&mut self, element: &quick_xml::events::BytesStart<'_>) -> Result<()> {
        match element.local_name().as_ref() {
            b"sheetData" => {
                if self.saw_sheet_data {
                    bail!("worksheet contains duplicate sheetData elements");
                }
                self.saw_sheet_data = true;
                Ok(())
            }
            b"row" if self.in_sheet_data => self.empty_row(element),
            b"c" if self.cell_open => bail!("worksheet contains nested cells"),
            b"c" if self.row_open => self.empty_cell(element),
            b"c" => bail!("worksheet cell is outside a row"),
            b"dimension" => self.dimension(element),
            _ => Ok(()),
        }
    }

    fn end(&mut self, name: &[u8]) -> Result<()> {
        match name {
            b"c" => {
                if !self.cell_open {
                    bail!("worksheet cell ended without a cell start");
                }
                self.cell_open = false;
                Ok(())
            }
            b"row" => {
                if !self.row_open || self.cell_open {
                    bail!("worksheet row ended before its cells");
                }
                self.row_open = false;
                self.row_number = None;
                self.last_column = None;
                Ok(())
            }
            b"sheetData" => {
                if !self.in_sheet_data || self.row_open || self.cell_open {
                    bail!("worksheet sheetData ended prematurely");
                }
                self.in_sheet_data = false;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn dimension(&mut self, element: &quick_xml::events::BytesStart<'_>) -> Result<()> {
        if self.declared_dimension.is_some() {
            bail!("worksheet contains duplicate dimensions");
        }
        self.declared_dimension = cells::attribute(element, b"ref")?;
        Ok(())
    }

    fn start_row(&mut self, element: &quick_xml::events::BytesStart<'_>) -> Result<()> {
        if self.row_open {
            bail!("worksheet contains nested rows");
        }
        let row = row_number(cells::attribute(element, b"r")?, self.last_row)?;
        self.row_open = true;
        self.row_number = Some(row);
        self.last_row = Some(row);
        self.last_column = None;
        self.rows = self
            .rows
            .checked_add(1)
            .context("worksheet row count overflow")?;
        Ok(())
    }

    fn empty_row(&mut self, element: &quick_xml::events::BytesStart<'_>) -> Result<()> {
        if self.row_open {
            bail!("worksheet contains nested rows");
        }
        let row = row_number(cells::attribute(element, b"r")?, self.last_row)?;
        self.last_row = Some(row);
        self.rows = self
            .rows
            .checked_add(1)
            .context("worksheet row count overflow")?;
        Ok(())
    }

    fn start_cell(&mut self, element: &quick_xml::events::BytesStart<'_>) -> Result<()> {
        if self.cell_open {
            bail!("worksheet contains nested cells");
        }
        let (_, column) = self.cell_position(element)?;
        if Some(column) <= self.last_column {
            bail!("worksheet cells are duplicate or out of order");
        }
        self.last_column = Some(column);
        self.cell_open = true;
        Ok(())
    }

    fn empty_cell(&mut self, element: &quick_xml::events::BytesStart<'_>) -> Result<()> {
        let (_, column) = self.cell_position(element)?;
        if Some(column) <= self.last_column {
            bail!("worksheet cells are duplicate or out of order");
        }
        self.last_column = Some(column);
        Ok(())
    }

    fn cell_position(&self, element: &quick_xml::events::BytesStart<'_>) -> Result<(u32, usize)> {
        let reference =
            cells::attribute(element, b"r")?.context("worksheet cell is missing reference")?;
        let row = self.row_number.context("worksheet cell has no row")?;
        let column = cells::validate_cell_row(&reference, row)?;
        Ok((row, column))
    }

    pub(super) fn finish(self, path: &str) -> Result<WorksheetPreflight> {
        if !self.saw_sheet_data || self.in_sheet_data || self.row_open || self.cell_open {
            bail!("worksheet {path} has incomplete sheetData");
        }
        Ok(WorksheetPreflight {
            path: path.to_owned(),
            declared_dimension: self.declared_dimension,
            xml_rows: self.rows,
        })
    }
}

fn row_number(value: Option<String>, previous: Option<u32>) -> Result<u32> {
    let fallback = match previous {
        Some(row) => u64::from(row)
            .checked_add(1)
            .context("worksheet row number overflow")?,
        None => 1,
    };
    let row = value.map_or_else(
        || u32::try_from(fallback).context("worksheet row count exceeds Excel range"),
        |raw| {
            raw.parse::<u32>()
                .with_context(|| format!("invalid Excel row number {raw:?}"))
        },
    )?;
    if row == 0 || row > 1_048_576 {
        bail!("invalid Excel row number {row}");
    }
    if previous.is_some_and(|prior| row <= prior) {
        bail!("worksheet row position is duplicate or out of order");
    }
    Ok(row)
}

#[derive(Debug, Default)]
pub(super) struct SharedStringsGuard {
    count: usize,
    item_bytes: usize,
    in_item: bool,
}

impl SharedStringsGuard {
    pub(super) fn observe(&mut self, event: &Event<'_>) -> Result<()> {
        match event {
            Event::Start(element) if element.local_name().as_ref() == b"sst" => {
                if let Some(raw) = cells::attribute(element, b"uniqueCount")? {
                    let count = raw
                        .parse::<usize>()
                        .with_context(|| format!("invalid shared-string uniqueCount {raw:?}"))?;
                    if count > 2_000_000 {
                        bail!("shared-string count exceeds 2000000");
                    }
                }
                Ok(())
            }
            Event::Start(element) if element.local_name().as_ref() == b"si" => {
                if self.in_item {
                    bail!("shared strings contain nested si elements");
                }
                self.in_item = true;
                self.item_bytes = 0;
                self.count = self
                    .count
                    .checked_add(1)
                    .context("shared-string count overflow")?;
                if self.count > 2_000_000 {
                    bail!("shared-string count exceeds 2000000");
                }
                Ok(())
            }
            Event::Text(text) if self.in_item => self.add_bytes(text.len()),
            Event::CData(text) if self.in_item => self.add_bytes(text.len()),
            Event::GeneralRef(reference) if self.in_item => {
                self.add_bytes(cells::decode_reference(reference)?.len_utf8())
            }
            Event::Empty(element) if element.local_name().as_ref() == b"si" => {
                if self.in_item {
                    bail!("shared strings contain nested si elements");
                }
                self.count = self
                    .count
                    .checked_add(1)
                    .context("shared-string count overflow")?;
                if self.count > 2_000_000 {
                    bail!("shared-string count exceeds 2000000");
                }
                Ok(())
            }
            Event::End(element) if element.local_name().as_ref() == b"si" => {
                if !self.in_item {
                    bail!("shared-string si ended without a start");
                }
                self.in_item = false;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn add_bytes(&mut self, bytes: usize) -> Result<()> {
        self.item_bytes = self
            .item_bytes
            .checked_add(bytes)
            .context("shared-string size overflow")?;
        if self.item_bytes > 4 * 1024 * 1024 {
            bail!("shared string exceeds 4194304 bytes");
        }
        Ok(())
    }

    pub(super) fn finish(self) -> Result<()> {
        if self.in_item {
            bail!("shared strings XML ended inside si");
        }
        Ok(())
    }
}
