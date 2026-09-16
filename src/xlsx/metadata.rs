use super::cells::{attribute, decode_xml_text};
use super::xml::XmlDocument;
use super::{MAX_WORKSHEETS, MAX_ZIP_ENTRY_BYTES, REL_WORKSHEET};
use anyhow::{bail, Context, Result};
use quick_xml::{events::Event, Reader};
use std::{
    collections::HashMap,
    fs::File,
    io::{Read, Seek},
    path::Path,
};
use zip::{read::ZipFile, ZipArchive};

#[derive(Debug, Clone)]
pub(crate) struct Relationship {
    pub(crate) target: String,
    pub(crate) relation_type: String,
}

#[derive(Debug, Clone)]
pub(crate) struct SheetMeta {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) sheet_id: u32,
}

pub(crate) struct BoundedReader<R> {
    inner: R,
    consumed: u64,
    limit: u64,
}

impl<R> BoundedReader<R> {
    pub(crate) fn new(inner: R, limit: u64) -> Self {
        Self {
            inner,
            consumed: 0,
            limit,
        }
    }
}

impl<R: Read> Read for BoundedReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let remaining = self.limit.saturating_sub(self.consumed);
        if remaining == 0 {
            let mut probe = [0_u8; 1];
            let count = self.inner.read(&mut probe)?;
            if count != 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "decompressed ZIP entry exceeds byte limit",
                ));
            }
            return Ok(0);
        }
        let length = buffer.len().min(match usize::try_from(remaining) {
            Ok(value) => value,
            Err(_) => usize::MAX,
        });
        let target = buffer
            .get_mut(..length)
            .ok_or_else(|| std::io::Error::other("bounded reader buffer slice is invalid"))?;
        let count = self.inner.read(target)?;
        self.consumed = self
            .consumed
            .saturating_add(u64::try_from(count).unwrap_or(u64::MAX));
        Ok(count)
    }
}

pub(crate) fn bounded_entry<R: Read + ?Sized>(
    entry: ZipFile<'_, R>,
) -> Result<BoundedReader<ZipFile<'_, R>>> {
    if entry.size() > MAX_ZIP_ENTRY_BYTES {
        bail!("ZIP entry exceeds the {} byte limit", MAX_ZIP_ENTRY_BYTES);
    }
    Ok(BoundedReader::new(entry, MAX_ZIP_ENTRY_BYTES))
}

pub(crate) fn load_sheet_metadata(path: &Path) -> Result<Vec<SheetMeta>> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let workbook_xml = read_zip_string(&mut archive, "xl/workbook.xml")?;
    let relationships_xml = read_zip_string(&mut archive, "xl/_rels/workbook.xml.rels")?;
    let relationships = parse_relationships(&relationships_xml)?;
    let mut reader = Reader::from_str(&workbook_xml);
    let mut buffer = Vec::new();
    let mut document = XmlDocument::default();
    let mut sheets = Vec::new();
    loop {
        let event = reader.read_event_into(&mut buffer)?;
        let eof = matches!(&event, Event::Eof);
        if let Event::Empty(element) | Event::Start(element) = &event {
            if element.name().as_ref() == b"sheet" {
                if sheets.len() >= MAX_WORKSHEETS {
                    bail!("worksheet count exceeds the {} entry limit", MAX_WORKSHEETS);
                }
                let name = attribute(element, b"name")?.unwrap_or_else(String::new);
                let sheet_id = parse_sheet_id(attribute(element, b"sheetId")?)?;
                let relation_id = sheet_relation_id(element)?;
                let relationship = relationships.get(&relation_id).with_context(|| {
                    format!("missing worksheet relationship {relation_id} for {name}")
                })?;
                if is_worksheet_relationship(&relationship.relation_type) {
                    sheets.push(SheetMeta {
                        name,
                        path: normalize_zip_target(&relationship.target),
                        sheet_id,
                    });
                }
            }
        }
        document.observe(&event)?;
        buffer.clear();
        if eof {
            break;
        }
    }
    document.finish("workbook")?;
    Ok(sheets)
}

fn sheet_relation_id(event: &quick_xml::events::BytesStart<'_>) -> Result<String> {
    event
        .attributes()
        .with_checks(true)
        .try_fold(None, |found, item| -> Result<Option<String>> {
            let attribute = item?;
            if attribute.key.as_ref() == b"r:id" || attribute.key.as_ref().ends_with(b":id") {
                Ok(Some(decode_xml_text(attribute.value.as_ref())?))
            } else {
                Ok(found)
            }
        })?
        .with_context(|| "worksheet is missing relationship id")
}

fn parse_sheet_id(value: Option<String>) -> Result<u32> {
    value.map_or(Ok(0), |raw| {
        raw.parse::<u32>()
            .with_context(|| format!("invalid worksheet id {raw:?}"))
    })
}

fn is_worksheet_relationship(relation_type: &str) -> bool {
    relation_type == REL_WORKSHEET || relation_type.ends_with("/worksheet")
}

pub(crate) fn parse_relationships(xml: &str) -> Result<HashMap<String, Relationship>> {
    let mut reader = Reader::from_str(xml);
    let mut buffer = Vec::new();
    let mut document = XmlDocument::default();
    let mut relationships = HashMap::new();
    loop {
        let event = reader.read_event_into(&mut buffer)?;
        match &event {
            Event::Empty(element) | Event::Start(element)
                if element.name().as_ref() == b"Relationship" =>
            {
                let id =
                    attribute(element, b"Id")?.with_context(|| "relationship is missing Id")?;
                let target = attribute(element, b"Target")?
                    .with_context(|| format!("relationship {id} is missing Target"))?;
                let relation_type = attribute(element, b"Type")?
                    .with_context(|| format!("relationship {id} is missing Type"))?;
                if relationships
                    .insert(
                        id.clone(),
                        Relationship {
                            target,
                            relation_type,
                        },
                    )
                    .is_some()
                {
                    bail!("duplicate relationship id {id}");
                }
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
    document.finish("Relationships")?;
    Ok(relationships)
}

fn normalize_zip_target(target: &str) -> String {
    if target.starts_with('/') {
        target.trim_start_matches('/').to_owned()
    } else if target.starts_with("xl/") {
        target.to_owned()
    } else {
        format!("xl/{target}")
    }
}

pub(crate) fn read_zip_string<R: Read + Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<String> {
    let file = archive.by_name(name)?;
    let mut bounded = bounded_entry(file)?;
    let mut bytes = Vec::new();
    bounded.read_to_end(&mut bytes)?;
    String::from_utf8(bytes).with_context(|| format!("ZIP entry {name} is not UTF-8"))
}
