mod zip;

use super::guards::{SharedStringsGuard, WorksheetGuard};
use crate::xlsx::{cells, xml::XmlDocument};
use ::zip::{read::ZipFile, ZipArchive};
use anyhow::{bail, Context, Result};
use quick_xml::{events::Event, Reader};
use std::{
    fs::File,
    io::{BufReader, Read},
};

#[derive(Debug, Clone)]
pub(crate) struct WorksheetPreflight {
    pub(crate) path: String,
    pub(crate) declared_dimension: Option<String>,
    pub(crate) xml_rows: u64,
}

const MAX_ZIP_ENTRY_BYTES: u64 = 512 * 1024 * 1024;
const MAX_SHARED_STRING_TOTAL_BYTES: u64 = 256 * 1024 * 1024;
const MAX_TOTAL_DECOMPRESSED_BYTES: u64 = 1024 * 1024 * 1024;
const MAX_ZIP_ENTRIES: usize = 4096;

#[derive(Debug, Default)]
pub(crate) struct Report {
    pub(crate) worksheets: Vec<WorksheetPreflight>,
}

struct CountingReader<'a, R> {
    inner: R,
    total: &'a mut u64,
    consumed: u64,
    limit: u64,
}

impl<R: Read> Read for CountingReader<'_, R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        let remaining_entry = self.limit.saturating_sub(self.consumed);
        let remaining_total = MAX_TOTAL_DECOMPRESSED_BYTES.saturating_sub(*self.total);
        let remaining = remaining_entry.min(remaining_total);
        if remaining == 0 {
            let mut probe = [0_u8; 1];
            let count = self.inner.read(&mut probe)?;
            if count != 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "decompressed ZIP data exceeds a configured limit",
                ));
            }
            return Ok(0);
        }
        let size = usize::try_from(remaining)
            .map_err(|_| std::io::Error::other("bounded ZIP read exceeds address space"))?
            .min(buffer.len());
        let target = buffer
            .get_mut(..size)
            .ok_or_else(|| std::io::Error::other("invalid bounded reader buffer"))?;
        let count = self.inner.read(target)?;
        let amount = u64::try_from(count).map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "decompressed byte count overflow",
            )
        })?;
        self.consumed = self.consumed.checked_add(amount).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "decompressed byte count overflow",
            )
        })?;
        *self.total = self.total.checked_add(amount).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "decompressed byte count overflow",
            )
        })?;
        Ok(count)
    }
}

pub(crate) fn inspect(path: &std::path::Path) -> Result<Report> {
    zip::validate(path)?;
    let mut archive = ZipArchive::new(File::open(path).context("open workbook ZIP")?)
        .context("read workbook ZIP")?;
    let mut report = Report::default();
    let mut total = 0_u64;
    (0..archive.len()).try_for_each(|index| {
        let mut entry = archive
            .by_index(index)
            .with_context(|| format!("read ZIP entry {index}"))?;
        inspect_entry(&mut entry, &mut total, &mut report)
    })?;
    Ok(report)
}

fn inspect_entry<R: Read>(
    entry: &mut ZipFile<'_, R>,
    total: &mut u64,
    report: &mut Report,
) -> Result<()> {
    let name = entry.name().to_owned();
    if entry.is_dir() {
        return Ok(());
    }
    if entry.size() > MAX_ZIP_ENTRY_BYTES {
        bail!("ZIP entry {name} exceeds the 512 MiB limit");
    }
    let shared = name.ends_with("sharedStrings.xml");
    if shared && entry.size() > MAX_SHARED_STRING_TOTAL_BYTES {
        bail!("shared-string XML exceeds the 256 MiB limit");
    }
    let counted = CountingReader {
        inner: entry,
        total,
        consumed: 0,
        limit: if shared {
            MAX_SHARED_STRING_TOTAL_BYTES
        } else {
            MAX_ZIP_ENTRY_BYTES
        },
    };
    if name.ends_with(".xml") || name.ends_with(".rels") {
        scan_xml(counted, &name, shared, report)
    } else {
        consume(counted, &name)
    }
}

fn consume<R: Read>(mut source: R, name: &str) -> Result<()> {
    std::io::copy(&mut source, &mut std::io::sink())
        .with_context(|| format!("read ZIP entry {name}"))?;
    Ok(())
}

fn scan_xml<R: Read>(source: R, path: &str, shared: bool, report: &mut Report) -> Result<()> {
    let mut reader = Reader::from_reader(BufReader::new(source));
    reader.config_mut().trim_text(false);
    reader.config_mut().enable_all_checks(true);
    let mut buffer = Vec::new();
    let mut root = None;
    let mut document = XmlDocument::default();
    let mut guard: Option<WorksheetGuard> = None;
    let mut shared_guard = shared.then(SharedStringsGuard::default);
    loop {
        buffer.clear();
        let event = reader
            .read_event_into(&mut buffer)
            .context("parse XML ZIP entry")?;
        validate_attributes(&event)?;
        if root.is_none() {
            root = root_name(&event);
            if root
                .as_deref()
                .is_some_and(|name| name.rsplit(|byte| *byte == b':').next() == Some(b"worksheet"))
            {
                guard = Some(WorksheetGuard::default());
            }
        }
        if let Some(value) = guard.as_mut() {
            value.observe(&event)?;
        }
        if let Some(value) = shared_guard.as_mut() {
            value.observe(&event)?;
        }
        match &event {
            Event::DocType(_) => bail!("DOCTYPE and external XML entities are unsupported"),
            Event::Text(text) => validate_xml_text(text.as_ref())?,
            Event::CData(text) => validate_xml_text(text.as_ref())?,
            Event::GeneralRef(reference) => {
                cells::decode_reference(reference)?;
            }
            _ => {}
        }
        document.observe(&event)?;
        if matches!(event, Event::Eof) {
            break;
        }
    }
    let root = root.context("XML entry has no root element")?;
    let root = String::from_utf8(root).context("XML root name is not UTF-8")?;
    document.finish(&root)?;
    if let Some(value) = guard {
        report.worksheets.push(value.finish(path)?);
    }
    if let Some(value) = shared_guard {
        value.finish()?;
    }
    Ok(())
}
fn root_name(event: &Event<'_>) -> Option<Vec<u8>> {
    match event {
        Event::Start(element) | Event::Empty(element) => Some(element.name().as_ref().to_vec()),
        _ => None,
    }
}

fn validate_xml_text(bytes: &[u8]) -> Result<()> {
    let text = std::str::from_utf8(bytes).context("XML text is not UTF-8")?;
    if text.chars().any(|character| {
        !matches!(
            character,
            '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..
        )
    }) {
        bail!("XML text contains a forbidden character");
    }
    Ok(())
}

fn validate_attributes(event: &Event<'_>) -> Result<()> {
    let element = match event {
        Event::Start(value) | Event::Empty(value) => value,
        _ => return Ok(()),
    };
    element.attributes().with_checks(true).try_for_each(|item| {
        let attribute = item?;
        let text = cells::decode_xml_text(attribute.value.as_ref())?;
        validate_xml_text(text.as_bytes())
    })
}
