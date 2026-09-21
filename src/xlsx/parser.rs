mod rows;

use super::metadata::{bounded_entry, load_sheet_metadata};
use super::selection::select_sheets;
use super::xml::XmlDocument;
use super::{
    MAX_SHARED_STRINGS, MAX_SHARED_STRING_BYTES, MAX_SHARED_STRING_TOTAL_BYTES, MAX_ZIP_ENTRY_BYTES,
};
use crate::model::{SourceRecord, WorkbookStats};
use anyhow::{bail, Context, Result};
use quick_xml::{events::Event, Reader};
use std::{fs::File, io::BufReader, path::Path};
use zip::ZipArchive;

pub(crate) use rows::parse_worksheet;

pub fn visit_records(
    path: &Path,
    visitor: impl FnMut(SourceRecord) -> Result<()>,
) -> Result<WorkbookStats> {
    walk_records(path, &super::ScanMode::Exhaustive, visitor)
}

pub(crate) fn walk_records<F>(
    path: &Path,
    mode: &super::ScanMode,
    mut visitor: F,
) -> Result<WorkbookStats>
where
    F: FnMut(SourceRecord) -> Result<()>,
{
    let shared_strings = load_shared_strings(path)?;
    let sheets = select_sheets(load_sheet_metadata(path)?, mode)?;
    sheets
        .into_iter()
        .try_fold(WorkbookStats::default(), |mut stats, sheet| {
            let mut archive = ZipArchive::new(File::open(path)?)?;
            let file = archive
                .by_name(&sheet.path)
                .with_context(|| format!("opening {} in workbook", sheet.path))?;
            let bounded = bounded_entry(file)
                .with_context(|| format!("opening bounded worksheet {}", sheet.path))?;
            let sheet_stats = parse_worksheet(bounded, &sheet.name, &shared_strings, &mut visitor)?;
            stats.actual_data_rows = stats
                .actual_data_rows
                .saturating_add(sheet_stats.actual_data_rows);
            stats.sheets.push(sheet_stats);
            Ok(stats)
        })
}

pub(crate) fn load_shared_strings(path: &Path) -> Result<Vec<String>> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let file = match archive.by_name("xl/sharedStrings.xml") {
        Ok(file) => file,
        Err(zip::result::ZipError::FileNotFound) => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    if file.size() > MAX_ZIP_ENTRY_BYTES {
        bail!(
            "sharedStrings.xml exceeds the {} byte limit",
            MAX_ZIP_ENTRY_BYTES
        );
    }
    let mut reader = Reader::from_reader(BufReader::new(bounded_entry(file)?));
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut document = XmlDocument::default();
    let mut strings = SharedStrings::default();
    loop {
        let event = reader.read_event_into(&mut buffer)?;
        strings.observe(&event)?;
        let eof = matches!(&event, Event::Eof);
        document.observe(&event)?;
        buffer.clear();
        if eof {
            break;
        }
    }
    document.finish("sst")?;
    Ok(strings.into_strings())
}

/// Shared-string table accumulated one XML event at a time.
#[derive(Default)]
struct SharedStrings {
    strings: Vec<String>,
    current: String,
    total_bytes: usize,
    in_item: bool,
    in_text: bool,
}

impl SharedStrings {
    fn observe(&mut self, event: &Event<'_>) -> Result<()> {
        match event {
            Event::Start(element) if element.name().as_ref() == b"si" => {
                self.in_item = true;
                self.current.clear();
            }
            Event::Start(element) if self.in_item && element.name().as_ref() == b"t" => {
                self.in_text = true;
            }
            Event::Text(text) if self.in_item && self.in_text => {
                append_text(
                    &mut self.current,
                    &super::cells::decode_xml_text(text.as_ref())?,
                )?;
            }
            Event::CData(text) if self.in_item && self.in_text => {
                append_text(&mut self.current, std::str::from_utf8(text.as_ref())?)?;
            }
            Event::GeneralRef(reference) if self.in_item && self.in_text => {
                let character = super::cells::decode_reference(reference)?;
                let mut bytes = [0_u8; 4];
                append_text(&mut self.current, character.encode_utf8(&mut bytes))?;
            }
            Event::End(element) if element.name().as_ref() == b"t" => self.in_text = false,
            Event::End(element) if element.name().as_ref() == b"si" => {
                self.finish_item()?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Closes the current `<si>` item, enforcing the count and byte budgets.
    fn finish_item(&mut self) -> Result<()> {
        if self.strings.len() >= MAX_SHARED_STRINGS {
            bail!(
                "shared string count exceeds the {} entry limit",
                MAX_SHARED_STRINGS
            );
        }
        self.total_bytes = self
            .total_bytes
            .checked_add(self.current.len())
            .context("shared string byte count overflow")?;
        if self.total_bytes > MAX_SHARED_STRING_TOTAL_BYTES {
            bail!(
                "shared string bytes exceed the {} byte limit",
                MAX_SHARED_STRING_TOTAL_BYTES
            );
        }
        self.strings.push(std::mem::take(&mut self.current));
        self.in_item = false;
        Ok(())
    }

    fn into_strings(self) -> Vec<String> {
        self.strings
    }
}

fn append_text(current: &mut String, text: &str) -> Result<()> {
    let next = current
        .len()
        .checked_add(text.len())
        .context("shared string byte count overflow")?;
    if next > MAX_SHARED_STRING_BYTES {
        bail!(
            "shared string exceeds the {} byte limit",
            MAX_SHARED_STRING_BYTES
        );
    }
    current.push_str(text);
    Ok(())
}
