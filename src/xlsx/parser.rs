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
    let mut strings = Vec::new();
    let mut total_bytes = 0_usize;
    let mut in_item = false;
    let mut in_text = false;
    let mut current = String::new();
    loop {
        let event = reader.read_event_into(&mut buffer)?;
        match &event {
            Event::Start(element) if element.name().as_ref() == b"si" => {
                in_item = true;
                current.clear();
            }
            Event::Start(element) if in_item && element.name().as_ref() == b"t" => in_text = true,
            Event::Text(text) if in_item && in_text => {
                append_text(&mut current, &super::cells::decode_xml_text(text.as_ref())?)?;
            }
            Event::CData(text) if in_item && in_text => {
                append_text(&mut current, std::str::from_utf8(text.as_ref())?)?;
            }
            Event::GeneralRef(reference) if in_item && in_text => {
                let character = super::cells::decode_reference(reference)?;
                let mut bytes = [0_u8; 4];
                append_text(&mut current, character.encode_utf8(&mut bytes))?;
            }
            Event::End(element) if element.name().as_ref() == b"t" => in_text = false,
            Event::End(element) if element.name().as_ref() == b"si" => {
                if strings.len() >= MAX_SHARED_STRINGS {
                    bail!(
                        "shared string count exceeds the {} entry limit",
                        MAX_SHARED_STRINGS
                    );
                }
                total_bytes = total_bytes
                    .checked_add(current.len())
                    .context("shared string byte count overflow")?;
                if total_bytes > MAX_SHARED_STRING_TOTAL_BYTES {
                    bail!(
                        "shared string bytes exceed the {} byte limit",
                        MAX_SHARED_STRING_TOTAL_BYTES
                    );
                }
                strings.push(std::mem::take(&mut current));
                in_item = false;
            }
            _ => {}
        }
        let eof = matches!(&event, Event::Eof);
        document.observe(&event)?;
        buffer.clear();
        if eof {
            break;
        }
    }
    document.finish("sst")?;
    Ok(strings)
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
