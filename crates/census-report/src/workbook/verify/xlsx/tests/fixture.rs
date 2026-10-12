use crate::report::ReportResult;
use crate::workbook::verify::canonical::Value;
use crate::workbook::verify::read::tests::TestResult;
use crate::workbook::verify::read::{EXCEL_COLUMNS_PER_SHEET, EXCEL_ROWS_PER_SHEET};
use crate::workbook::verify::xlsx::{Archive, Cells, Profile};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>";
const LOCAL_SIGNATURE: u32 = 0x0403_4b50;
const CENTRAL_SIGNATURE: u32 = 0x0201_4b50;
const END_SIGNATURE: u32 = 0x0605_4b50;
const STORED: u16 = 0;
const DEFLATED: u16 = 8;

pub(super) fn token_profile(token: usize) -> Profile {
    Profile {
        container: 1 << 20,
        member: 1 << 16,
        inflated: 1 << 16,
        token,
        cells: 1 << 10,
        members: 8,
    }
}

pub(super) fn byte_profile(inflated: u64) -> Profile {
    Profile {
        container: 1 << 20,
        member: 1 << 17,
        inflated,
        token: 1 << 16,
        cells: 1 << 12,
        members: 8,
    }
}

pub(super) fn archive(entries: &[(&str, &[u8], u16, u32)]) -> Vec<u8> {
    let mut file = Vec::new();
    let mut central = Vec::new();
    for &(name, data, method, uncompressed) in entries {
        let offset = u32::try_from(file.len()).map_or(u32::MAX, |length| length);
        let compressed = u32::try_from(data.len()).map_or(u32::MAX, |length| length);
        let crc = crc32(data);
        push32(&mut file, LOCAL_SIGNATURE);
        push16(&mut file, 20);
        push16(&mut file, 0);
        push16(&mut file, method);
        push16(&mut file, 0);
        push16(&mut file, 0);
        push32(&mut file, crc);
        push32(&mut file, compressed);
        push32(&mut file, uncompressed);
        push16(
            &mut file,
            u16::try_from(name.len()).map_or(u16::MAX, |length| length),
        );
        push16(&mut file, 0);
        file.extend_from_slice(name.as_bytes());
        file.extend_from_slice(data);
        push32(&mut central, CENTRAL_SIGNATURE);
        push16(&mut central, 20);
        push16(&mut central, 20);
        push16(&mut central, 0);
        push16(&mut central, method);
        push16(&mut central, 0);
        push16(&mut central, 0);
        push32(&mut central, crc);
        push32(&mut central, compressed);
        push32(&mut central, uncompressed);
        push16(
            &mut central,
            u16::try_from(name.len()).map_or(u16::MAX, |length| length),
        );
        push16(&mut central, 0);
        push16(&mut central, 0);
        push16(&mut central, 0);
        push16(&mut central, 0);
        push32(&mut central, 0);
        push32(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }
    let directory = u32::try_from(file.len()).map_or(u32::MAX, |length| length);
    let size = u32::try_from(central.len()).map_or(u32::MAX, |length| length);
    let count = u16::try_from(entries.len()).map_or(u16::MAX, |length| length);
    file.extend_from_slice(&central);
    push32(&mut file, END_SIGNATURE);
    push16(&mut file, 0);
    push16(&mut file, 0);
    push16(&mut file, count);
    push16(&mut file, count);
    push32(&mut file, size);
    push32(&mut file, directory);
    push16(&mut file, 0);
    file
}

pub(super) fn workbook_archive(sheet: &[u8], method: u16, declared: u32) -> Vec<u8> {
    let content_types = format!(
        "{DECLARATION}<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"xml\" ContentType=\"application/xml\"/></Types>"
    );
    let root_rels = format!(
        "{DECLARATION}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>"
    );
    let workbook = format!(
        "{DECLARATION}<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets><sheet name=\"Sheet1\" sheetId=\"1\" r:id=\"rId1\"/></sheets></workbook>"
    );
    let rels = format!(
        "{DECLARATION}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"/></Relationships>"
    );
    let parts: Vec<(&str, &[u8], u16, u32)> = vec![
        (
            "[Content_Types].xml",
            content_types.as_bytes(),
            STORED,
            size(&content_types),
        ),
        (
            "_rels/.rels",
            root_rels.as_bytes(),
            STORED,
            size(&root_rels),
        ),
        (
            "xl/workbook.xml",
            workbook.as_bytes(),
            STORED,
            size(&workbook),
        ),
        (
            "xl/_rels/workbook.xml.rels",
            rels.as_bytes(),
            STORED,
            size(&rels),
        ),
        ("xl/worksheets/sheet1.xml", sheet, method, declared),
    ];
    archive(&parts)
}

pub(super) fn workbook(sheet: &str) -> Vec<u8> {
    workbook_archive(sheet.as_bytes(), STORED, size(sheet))
}

pub(super) fn deflated_workbook(sheet: &str) -> Vec<u8> {
    let stream = deflate_stored(sheet.as_bytes());
    workbook_archive(&stream, DEFLATED, size(sheet))
}

pub(super) fn deflate_stored(data: &[u8]) -> Vec<u8> {
    let length = u16::try_from(data.len()).map_or(u16::MAX, |value| value);
    let mut stream = Vec::with_capacity(data.len().saturating_add(5));
    stream.push(0x01);
    stream.extend_from_slice(&length.to_le_bytes());
    stream.extend_from_slice(&(!length).to_le_bytes());
    stream.extend_from_slice(data);
    stream
}

pub(super) fn sheet_with(dimension: &str, body: &str) -> String {
    format!(
        "<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><dimension ref=\"{dimension}\"/><sheetData>{body}</sheetData></worksheet>"
    )
}

pub(super) fn row_with(cell: &str) -> String {
    sheet_with("A1", &format!("<row r=\"1\">{cell}</row>"))
}

pub(super) fn inline_sheet() -> String {
    sheet_with(
        "A1:C3",
        "<row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>Name</t></is></c><c r=\"B1\" t=\"inlineStr\"><is><t>Mark</t></is></c><c r=\"C1\" t=\"inlineStr\"><is><t>Team &amp; School</t></is></c></row><row r=\"2\"><c r=\"A2\" t=\"inlineStr\"><is><t xml:space=\"preserve\"> Oracle  Runner </t></is></c><c r=\"B2\"><v>11.5</v></c><c r=\"C2\" t=\"b\"><v>1</v></c></row>",
    )
}

pub(super) fn write(directory: &Path, name: &str, bytes: &[u8]) -> TestResult<PathBuf> {
    let path = directory.join(name);
    std::fs::write(&path, bytes)?;
    Ok(path)
}

#[derive(Default)]
pub(super) struct Walked {
    pub(super) cells: BTreeMap<(usize, usize), Value>,
    pub(super) rows: Option<usize>,
}

impl Cells for Walked {
    fn cell(&mut self, row: usize, column: usize, value: Value) -> ReportResult<()> {
        self.cells.insert((row, column), value);
        Ok(())
    }

    fn finish(&mut self, rows: usize) {
        self.rows = Some(rows);
    }
}

pub(super) fn walk(archive: &Archive, sheet: &str) -> TestResult<(Walked, Option<usize>)> {
    let mut walked = Walked::default();
    let written = archive.read_sheet(
        sheet,
        EXCEL_ROWS_PER_SHEET,
        EXCEL_COLUMNS_PER_SHEET,
        &mut walked,
    )?;
    Ok((walked, written))
}

pub(super) fn failure(archive: &Archive, sheet: &str) -> TestResult<crate::report::ReportError> {
    match archive.read_sheet(
        sheet,
        EXCEL_ROWS_PER_SHEET,
        EXCEL_COLUMNS_PER_SHEET,
        &mut Walked::default(),
    ) {
        Ok(_) => Err(format!("sheet {sheet} was read without complaint").into()),
        Err(error) => Ok(error),
    }
}

fn size(text: &str) -> u32 {
    u32::try_from(text.len()).map_or(u32::MAX, |length| length)
}

fn push16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFF_u32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}
