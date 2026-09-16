use super::*;
use crate::model::SOURCE_HEADERS;
use anyhow::Result;
use std::io::Write;
use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

fn fixture_workbook() -> Result<(tempfile::TempDir, std::path::PathBuf)> {
    let directory = tempdir()?;
    let path = directory.path().join("streaming.xlsx");
    let file = std::fs::File::create(&path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    write_entry(&mut zip, "xl/workbook.xml", workbook_xml(), options)?;
    write_entry(
        &mut zip,
        "xl/_rels/workbook.xml.rels",
        relationships_xml(),
        options,
    )?;
    write_entry(
        &mut zip,
        "[Content_Types].xml",
        r#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/worksheets/sheet2.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/><Override PartName="/xl/worksheets/sheet3.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#,
        options,
    )?;
    write_entry(
        &mut zip,
        "xl/worksheets/sheet1.xml",
        &source_sheet("Export", 5, true),
        options,
    )?;
    write_entry(
        &mut zip,
        "xl/worksheets/sheet2.xml",
        &source_sheet("Sheet1", 9, false),
        options,
    )?;
    write_entry(
        &mut zip,
        "xl/worksheets/sheet3.xml",
        &source_sheet("Athletic Matches", 2, true),
        options,
    )?;
    zip.finish()?;
    Ok((directory, path))
}

fn workbook_xml() -> &'static str {
    r#"<workbook xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Export" sheetId="1" r:id="rId1"/><sheet name="Sheet1" sheetId="2" r:id="rId2"/><sheet name="Athletic Matches" sheetId="3" r:id="rId3"/></sheets></workbook>"#
}

fn relationships_xml() -> &'static str {
    r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet3.xml"/></Relationships>"#
}

fn source_sheet(_name: &str, row: u32, populated_name: bool) -> String {
    let headers = SOURCE_HEADERS
        .iter()
        .enumerate()
        .map(|(column, header)| inline_cell(column, 1, header))
        .collect::<String>();
    let values = SOURCE_HEADERS
        .iter()
        .enumerate()
        .map(|(column, header)| {
            let value = match (*header, populated_name) {
                ("Person First", true) => "Ada",
                ("Person Last", true) => "Lovelace",
                ("Person First" | "Person Last", false) => "",
                ("Schools Name", _) => "Central High",
                ("Sports Sport", _) => "Track",
                _ => "preserved-value",
            };
            inline_cell(column, row, value)
        })
        .collect::<String>();
    format!(
        r#"<worksheet><dimension ref="A1:XFD999999"/><sheetData><row r="1">{headers}</row><row r="{row}">{values}</row></sheetData></worksheet>"#
    )
}

fn inline_cell(column: usize, row: u32, value: &str) -> String {
    format!(
        r#"<c r="{}{row}" t="inlineStr"><is><t>{value}</t></is></c>"#,
        column_name(column)
    )
}

fn write_entry<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    name: &str,
    contents: &str,
    options: SimpleFileOptions,
) -> Result<()> {
    zip.start_file(name, options)?;
    zip.write_all(contents.as_bytes())?;
    Ok(())
}

#[test]
fn visitor_streams_both_source_sheets_and_preserves_fields() -> Result<()> {
    let (_directory, path) = fixture_workbook()?;
    let mut records = Vec::new();
    let stats = visit_records(&path, |record| {
        records.push(record);
        Ok(())
    })?;
    assert_eq!(stats.sheets.len(), 2);
    assert_eq!(stats.actual_data_rows, 2);
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].source_key, "Export:5");
    assert_eq!(records[1].source_key, "Sheet1:9");
    assert_eq!(records[0].fields.len(), SOURCE_HEADERS.len());
    assert_eq!(
        records[0].fields.get("Person Email").map(String::as_str),
        Some("preserved-value")
    );
    assert_eq!(
        records[1].fields.get("Person First").map(String::as_str),
        Some("")
    );
    Ok(())
}

#[test]
fn visitor_propagates_callback_error_without_reading_remaining_rows() -> Result<()> {
    let (_directory, path) = fixture_workbook()?;
    let mut seen = 0_u32;
    let result = visit_records(&path, |_record| {
        seen = seen.saturating_add(1);
        anyhow::bail!("stop at callback")
    });
    assert!(result.is_err());
    assert_eq!(seen, 1);
    Ok(())
}

#[test]
fn visitor_rejects_duplicate_row_positions() -> Result<()> {
    let directory = tempdir()?;
    let path = directory.path().join("duplicate.xlsx");
    let file = std::fs::File::create(&path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    write_entry(
        &mut zip,
        "xl/workbook.xml",
        r#"<workbook xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Export" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        options,
    )?;
    write_entry(
        &mut zip,
        "xl/_rels/workbook.xml.rels",
        r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        options,
    )?;
    write_entry(
        &mut zip,
        "xl/worksheets/sheet1.xml",
        r#"<worksheet><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Person First</t></is></c></row><row r="2"><c r="A2" t="inlineStr"><is><t>Ada</t></is></c></row><row r="2"><c r="A2" t="inlineStr"><is><t>Again</t></is></c></row></sheetData></worksheet>"#,
        options,
    )?;
    zip.finish()?;
    let result = visit_records(&path, |_record| Ok(()));
    assert!(result.is_err());
    Ok(())
}

#[test]
fn visitor_rejects_truncated_worksheet_xml() -> Result<()> {
    let directory = tempdir()?;
    let path = directory.path().join("truncated.xlsx");
    let file = std::fs::File::create(&path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    write_entry(
        &mut zip,
        "xl/workbook.xml",
        r#"<workbook xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Export" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        options,
    )?;
    write_entry(
        &mut zip,
        "xl/_rels/workbook.xml.rels",
        r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        options,
    )?;
    write_entry(
        &mut zip,
        "xl/worksheets/sheet1.xml",
        "<worksheet><sheetData><row r=\"1\"><c r=\"A1\"><v>header",
        options,
    )?;
    zip.finish()?;
    let result = visit_records(&path, |_record| Ok(()));
    assert!(result.is_err());
    Ok(())
}

#[test]
fn bounded_reader_rejects_actual_decompressed_overflow() -> Result<()> {
    let mut reader = BoundedReader::new(std::io::Cursor::new(b"12345"), 4);
    let mut bytes = Vec::new();
    let result = std::io::Read::read_to_end(&mut reader, &mut bytes);
    assert!(result.is_err());
    assert_eq!(bytes, b"1234");
    Ok(())
}
#[test]
fn writer_preserves_source_fields_and_profile_link() -> Result<()> {
    let (_directory, input) = fixture_workbook()?;
    let output = input.with_file_name("enriched.xlsx");
    let record = crate::model::MatchRecord {
        source_key: "Export:5".to_owned(),
        prospect: crate::model::Prospect {
            source_key: "Export:5".to_owned(),
            sheet: "Export".to_owned(),
            excel_row: 5,
            source_fields: std::collections::BTreeMap::from([(
                "Person Email".to_owned(),
                "preserved@example.test".to_owned(),
            )]),
            ..Default::default()
        },
        selected_profile_url: "https://example.test/profile/7".to_owned(),
        ..Default::default()
    };
    append_matches_sheet(&input, &output, std::slice::from_ref(&record))?;
    let mut archive = zip::ZipArchive::new(std::fs::File::open(&output)?)?;
    let sheet = read_zip_string(&mut archive, "xl/worksheets/sheet3.xml")?;
    assert!(sheet.contains("preserved@example.test"));
    assert!(sheet.contains("HYPERLINK"));
    assert!(sheet.contains("profile/7"));
    Ok(())
}
