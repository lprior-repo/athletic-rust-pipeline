use super::super::cells::{cell_state, decode_xml_text, validate_cell_row, CellState};
use super::super::xml::XmlDocument;
use crate::model::{SheetStats, SourceRecord};
use anyhow::{bail, Context, Result};
use quick_xml::{events::Event, Reader};
use std::{
    collections::BTreeMap,
    io::{BufReader, Read},
};

pub(crate) fn parse_worksheet<R, F>(
    source: R,
    sheet_name: &str,
    shared_strings: &[String],
    mut on_record: F,
) -> Result<SheetStats>
where
    R: Read,
    F: FnMut(SourceRecord) -> Result<()>,
{
    let mut reader = Reader::from_reader(BufReader::new(source));
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut document = XmlDocument::default();
    let mut state = WorksheetState::new(sheet_name);
    loop {
        let event = reader.read_event_into(&mut buffer)?;
        state.handle(&event, shared_strings, &mut on_record)?;
        let eof = matches!(&event, Event::Eof);
        document.observe(&event)?;
        buffer.clear();
        if eof {
            break;
        }
    }
    document.finish("worksheet")?;
    state.finish()
}

struct WorksheetState {
    stats: SheetStats,
    headers: Vec<String>,
    header_row: Option<u32>,
    current_row_number: u32,
    last_row_number: Option<u32>,
    current_cells: BTreeMap<usize, String>,
    current_cell: Option<CellState>,
    row_has_data: bool,
    in_value: bool,
    in_inline_text: bool,
}

impl WorksheetState {
    fn new(sheet_name: &str) -> Self {
        Self {
            stats: SheetStats {
                name: sheet_name.to_owned(),
                ..Default::default()
            },
            headers: Vec::new(),
            header_row: None,
            current_row_number: 0,
            last_row_number: None,
            current_cells: BTreeMap::new(),
            current_cell: None,
            row_has_data: false,
            in_value: false,
            in_inline_text: false,
        }
    }

    fn handle<F>(
        &mut self,
        event: &Event<'_>,
        shared_strings: &[String],
        visitor: &mut F,
    ) -> Result<()>
    where
        F: FnMut(SourceRecord) -> Result<()>,
    {
        match event {
            Event::Empty(element) if element.name().as_ref() == b"dimension" => {
                self.stats.declared_dimension = super::super::cells::attribute(element, b"ref")?
            }
            Event::Empty(element) if element.name().as_ref() == b"row" => {
                self.empty_row(element)?
            }
            Event::Start(element) if element.name().as_ref() == b"row" => {
                self.start_row(element)?
            }
            Event::Empty(element) if element.name().as_ref() == b"c" => {
                self.empty_cell(element, shared_strings)?
            }
            Event::Start(element) if element.name().as_ref() == b"c" => self.start_cell(element)?,
            Event::Start(element) if element.name().as_ref() == b"v" => self.start_value()?,
            Event::Start(element) if element.name().as_ref() == b"t" => self.start_text()?,
            Event::Text(text) if self.in_value || self.in_inline_text => {
                self.append_cell_text(text.as_ref(), true)?
            }
            Event::CData(text) if self.in_value || self.in_inline_text => {
                self.append_cell_text(text.as_ref(), false)?
            }
            Event::GeneralRef(reference) if self.in_value || self.in_inline_text => {
                self.append_reference(reference)?
            }
            Event::End(element) if element.name().as_ref() == b"v" => self.end_value()?,
            Event::End(element) if element.name().as_ref() == b"t" => self.end_text()?,
            Event::End(element) if element.name().as_ref() == b"c" => {
                self.finish_cell(shared_strings)?
            }
            Event::End(element) if element.name().as_ref() == b"row" && self.row_has_data => {
                self.finish_row(visitor)?
            }
            _ => {}
        }
        Ok(())
    }

    fn empty_row(&mut self, element: &quick_xml::events::BytesStart<'_>) -> Result<()> {
        self.stats.xml_rows = self.stats.xml_rows.saturating_add(1);
        let fallback = next_row_number(self.last_row_number, self.stats.xml_rows)?;
        let row = worksheet_row_number(
            super::super::cells::attribute(element, b"r")?,
            fallback,
            self.last_row_number,
        )?;
        self.last_row_number = Some(row);
        Ok(())
    }

    fn start_row(&mut self, element: &quick_xml::events::BytesStart<'_>) -> Result<()> {
        self.stats.xml_rows = self.stats.xml_rows.saturating_add(1);
        let fallback = next_row_number(self.last_row_number, self.stats.xml_rows)?;
        self.current_row_number = worksheet_row_number(
            super::super::cells::attribute(element, b"r")?,
            fallback,
            self.last_row_number,
        )?;
        self.last_row_number = Some(self.current_row_number);
        self.current_cells.clear();
        self.current_cell = None;
        self.row_has_data = false;
        Ok(())
    }

    fn empty_cell(
        &mut self,
        element: &quick_xml::events::BytesStart<'_>,
        shared_strings: &[String],
    ) -> Result<()> {
        let cell = cell_state(element)?;
        let column = validate_cell_row(&cell.reference, self.current_row_number)?;
        let value = resolve_cell_value(&cell, shared_strings)?;
        self.row_has_data |= !value.is_empty();
        insert_cell(&mut self.current_cells, column, value)
    }

    fn start_cell(&mut self, element: &quick_xml::events::BytesStart<'_>) -> Result<()> {
        let cell = cell_state(element)?;
        validate_cell_row(&cell.reference, self.current_row_number)?;
        self.current_cell = Some(cell);
        Ok(())
    }

    fn start_value(&mut self) -> Result<()> {
        if self.current_cell.is_none() {
            bail!("worksheet value is outside a cell");
        }
        self.in_value = true;
        Ok(())
    }

    fn start_text(&mut self) -> Result<()> {
        if self.current_cell.is_none() {
            bail!("worksheet text is outside a cell");
        }
        self.in_inline_text = true;
        Ok(())
    }

    fn end_value(&mut self) -> Result<()> {
        self.in_value = false;
        Ok(())
    }
    fn end_text(&mut self) -> Result<()> {
        self.in_inline_text = false;
        Ok(())
    }

    fn append_cell_text(&mut self, text: &[u8], decode_entities: bool) -> Result<()> {
        let decoded = if decode_entities {
            decode_xml_text(text)?
        } else {
            std::str::from_utf8(text)?.to_owned()
        };
        match self.current_cell.as_mut() {
            Some(cell) => super::append_text(&mut cell.value, &decoded)?,
            None => bail!("worksheet text is outside a cell"),
        }
        Ok(())
    }

    fn append_reference(&mut self, reference: &quick_xml::events::BytesRef<'_>) -> Result<()> {
        let character = super::super::cells::decode_reference(reference)?;
        let mut bytes = [0_u8; 4];
        let text = character.encode_utf8(&mut bytes);
        let cell = self
            .current_cell
            .as_mut()
            .context("XML entity is outside a cell")?;
        super::append_text(&mut cell.value, text)
    }

    fn finish_cell(&mut self, shared_strings: &[String]) -> Result<()> {
        let cell = match self.current_cell.take() {
            Some(value) => value,
            None => bail!("worksheet cell ended without a cell start"),
        };
        let column = validate_cell_row(&cell.reference, self.current_row_number)?;
        let value = resolve_cell_value(&cell, shared_strings)?;
        self.row_has_data |= !value.is_empty();
        insert_cell(&mut self.current_cells, column, value)
    }

    fn finish_row<F>(&mut self, visitor: &mut F) -> Result<()>
    where
        F: FnMut(SourceRecord) -> Result<()>,
    {
        if self.header_row.is_none() {
            return self.finish_header();
        }
        self.stats.actual_data_rows = self.stats.actual_data_rows.saturating_add(1);
        self.stats.last_actual_row = self.current_row_number;
        let fields = self
            .headers
            .iter()
            .enumerate()
            .filter(|(_, header)| !header.is_empty())
            .map(|(column, header)| {
                let value = self
                    .current_cells
                    .get(&column)
                    .map_or_else(String::new, Clone::clone);
                (header.clone(), value)
            })
            .collect();
        visitor(SourceRecord {
            source_key: format!("{}:{}", self.stats.name, self.current_row_number),
            sheet: self.stats.name.clone(),
            excel_row: self.current_row_number,
            fields,
        })
    }

    fn finish_header(&mut self) -> Result<()> {
        let max_column = self
            .current_cells
            .keys()
            .next_back()
            .copied()
            .map_or(0, std::convert::identity);
        self.headers = (0..=max_column)
            .map(|column| {
                self.current_cells
                    .get(&column)
                    .map_or_else(String::new, Clone::clone)
            })
            .collect();
        self.header_row = Some(self.current_row_number);
        self.stats.headers = self.headers.clone();
        Ok(())
    }

    fn finish(self) -> Result<SheetStats> {
        if self.current_cell.is_some() || self.in_value || self.in_inline_text {
            bail!("worksheet ended with an incomplete cell");
        }
        Ok(self.stats)
    }
}

fn next_row_number(previous: Option<u32>, occurrence: u64) -> Result<u64> {
    previous.map_or(Ok(occurrence), |row| {
        u64::from(row)
            .checked_add(1)
            .context("worksheet row number overflow")
    })
}

fn worksheet_row_number(
    value: Option<String>,
    fallback: u64,
    previous: Option<u32>,
) -> Result<u32> {
    let row = value.map_or_else(
        || u32::try_from(fallback).context("worksheet row count exceeds Excel row range"),
        |raw| {
            raw.parse::<u32>()
                .with_context(|| format!("invalid Excel row number {raw:?}"))
        },
    )?;
    if row == 0 || row > super::super::MAX_EXCEL_ROW {
        bail!("invalid Excel row number {row}");
    }
    if previous.is_some_and(|prior| row <= prior) {
        bail!("worksheet row position {row} is duplicate or out of order");
    }
    Ok(row)
}

fn insert_cell(cells: &mut BTreeMap<usize, String>, column: usize, value: String) -> Result<()> {
    if cells.insert(column, value).is_some() {
        bail!("worksheet contains duplicate cell position in column {column}");
    }
    Ok(())
}

fn resolve_cell_value(cell: &CellState, shared_strings: &[String]) -> Result<String> {
    if cell.cell_type != "s" || cell.value.is_empty() {
        return Ok(cell.value.clone());
    }
    let index = cell
        .value
        .trim()
        .parse::<usize>()
        .with_context(|| format!("invalid shared string index {:?}", cell.value))?;
    match shared_strings.get(index) {
        Some(value) => Ok(value.clone()),
        None => bail!("shared string index {index} is out of range"),
    }
}
