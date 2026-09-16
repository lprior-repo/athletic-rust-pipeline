use super::*;
use anyhow::Result;
use std::io::Write;
use tempfile::tempdir;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

fn build_minimal_xlsx(
    dir: &tempfile::TempDir,
    name: &str,
    sheets: &[(&str, &str)],
) -> Result<std::path::PathBuf> {
    let path = dir.path().join(name);
    let file = std::fs::File::create(&path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

    let content_types = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
"#;
    let mut content_types_xml = String::from(content_types);
    for (index, (_sheet_name, _sheet_content)) in sheets.iter().enumerate() {
        let sheet_path = format!("xl/worksheets/sheet{}.xml", index + 1);
        content_types_xml.push_str(&format!(
            "<Override PartName=\"/{}\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>\n",
            sheet_path
        ));
    }
    content_types_xml.push_str("</Types>");
    write_entry(&mut zip, "[Content_Types].xml", &content_types_xml, options)?;
    write_entry(
        &mut zip,
        "xl/sharedStrings.xml",
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" count="1" uniqueCount="1">
<si><r><t>Shared </t></r><r><t>Family</t></r></si>
</sst>"#,
        options,
    )?;

    let workbook = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheets>
"#;
    let mut workbook_xml = String::from(workbook);
    for (index, (sheet_name, _sheet_content)) in sheets.iter().enumerate() {
        let r_id = format!("rId{}", index + 1);
        workbook_xml.push_str(&format!(
            "<sheet name=\"{}\" sheetId=\"{}\" r:id=\"{}\"/>\n",
            sheet_name,
            index + 1,
            r_id
        ));
    }
    workbook_xml.push_str("</sheets></workbook>");
    write_entry(&mut zip, "xl/workbook.xml", &workbook_xml, options)?;

    let rels_header = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
"#;
    let mut rels_xml = String::from(rels_header);
    for (index, (_sheet_name, _sheet_content)) in sheets.iter().enumerate() {
        let r_id = format!("rId{}", index + 1);
        let target = format!("worksheets/sheet{}.xml", index + 1);
        rels_xml.push_str(&format!(
            "<Relationship Id=\"{}\" Type=\"{}\" Target=\"{}\"/>\n",
            r_id, REL_WORKSHEET, target
        ));
    }
    rels_xml.push_str("</Relationships>");
    write_entry(&mut zip, "xl/_rels/workbook.xml.rels", &rels_xml, options)?;

    for (index, (_, sheet_content)) in sheets.iter().enumerate() {
        let sheet_path = format!("xl/worksheets/sheet{}.xml", index + 1);
        let sheet_xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
<dimension ref="A1:Z10"/>
<sheetData>
{}
</sheetData>
</worksheet>"#,
            sheet_content
        );
        write_entry(&mut zip, &sheet_path, &sheet_xml, options)?;
    }

    zip.finish()?;
    Ok(path)
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
fn first_worksheet_scan_retains_all_rows_and_fields() -> Result<()> {
    let dir = tempdir()?;
    let sheet1_content = r#"<row r="1"><c r="A1" t="inlineStr"><is><t>Person First</t></is></c><c r="B1" t="inlineStr"><is><t>Person Last</t></is></c><c r="C1" t="inlineStr"><is><t>Sports Sport</t></is></c><c r="D1" t="inlineStr"><is><t>Schools Name</t></is></c><c r="E1" t="inlineStr"><is><t>Address Mailing / Permanent City</t></is></c><c r="F1" t="inlineStr"><is><t>Address Mailing / Permanent Region</t></is></c></row>
<row r="2"><c r="A2" t="inlineStr"><is><t>Alice</t></is></c><c r="B2" t="inlineStr"><is><t>Smith</t></is></c><c r="C2" t="inlineStr"><is><t>Track</t></is></c><c r="D2" t="inlineStr"><is><t>High School</t></is></c><c r="E2" t="inlineStr"><is><t>City</t></is></c><c r="F2" t="inlineStr"><is><t>State</t></is></c></row>
<row r="3"><c r="A3" t="inlineStr"><is><t></t></is></c><c r="B3" t="inlineStr"><is><t>Blank</t></is></c><c r="C3" t="inlineStr"><is><t>Swim</t></is></c><c r="D3" t="inlineStr"><is><t>Other School</t></is></c><c r="E3" t="inlineStr"><is><t>Other City</t></is></c><c r="F3" t="inlineStr"><is><t>Other State</t></is></c></row>
<row r="4"><c r="A4" t="inlineStr"><is><t>Bob</t></is></c><c r="B4" t="inlineStr"><is><t></t></is></c><c r="C4" t="inlineStr"><is><t>Swim</t></is></c><c r="D4" t="inlineStr"><is><t>Another School</t></is></c><c r="E4" t="inlineStr"><is><t>Another City</t></is></c><c r="F4" t="inlineStr"><is><t>Another State</t></is></c></row>"#;

    let sheet2_content = r#"<row r="1"><c r="A1" t="inlineStr"><is><t>Person First</t></is></c><c r="B1" t="inlineStr"><is><t>Person Last</t></is></c><c r="C1" t="inlineStr"><is><t>Sports Sport</t></is></c><c r="D1" t="inlineStr"><is><t>Schools Name</t></is></c><c r="E1" t="inlineStr"><is><t>Address Mailing / Permanent City</t></is></c><c r="F1" t="inlineStr"><is><t>Address Mailing / Permanent Region</t></is></c></row>
<row r="2"><c r="A2" t="inlineStr"><is><t>Charlie</t></is></c><c r="B2" t="inlineStr"><is><t>Davis</t></is></c><c r="C2" t="inlineStr"><is><t>Swim</t></is></c><c r="D2" t="inlineStr"><is><t>Davis High</t></is></c><c r="E2" t="inlineStr"><is><t>Davis City</t></is></c><c r="F2" t="inlineStr"><is><t>Davis State</t></is></c></row>"#;

    let sheets = vec![("Export", sheet1_content), ("Sheet1", sheet2_content)];
    let path = build_minimal_xlsx(&dir, "fixture.xlsx", &sheets)?;

    let result = scan(&path, ScanMode::FirstWorksheet, None)?;

    assert_eq!(result.stats.sheets.len(), 1);
    assert_eq!(result.stats.sheets[0].name, "Export");
    assert_eq!(result.stats.actual_data_rows, 3);
    assert_eq!(result.stats.selected_prospects, 3);
    assert_eq!(result.prospects.len(), 3);

    let headers = &result.stats.sheets[0].headers;
    assert!(headers.contains(&"Person First".to_owned()));
    assert!(headers.contains(&"Person Last".to_owned()));
    assert!(headers.contains(&"Sports Sport".to_owned()));

    let first = &result.prospects[0];
    assert_eq!(first.first_name, "Alice");
    assert_eq!(first.last_name, "Smith");
    assert_eq!(first.sport, "Track");

    let second = &result.prospects[1];
    assert_eq!(second.first_name, "");
    assert_eq!(second.last_name, "Blank");
    assert_eq!(second.sport, "Swim");

    let third = &result.prospects[2];
    assert_eq!(third.first_name, "Bob");
    assert_eq!(third.last_name, "");
    assert_eq!(third.sport, "Swim");

    Ok(())
}

#[test]
fn exhaustive_mode_excludes_generated_sheets() -> Result<()> {
    let dir = tempdir()?;
    let sheet1_content = r#"<row r="1"><c r="A1" t="inlineStr"><is><t>Person First</t></is></c><c r="B1" t="inlineStr"><is><t>Person Last</t></is></c><c r="C1" t="inlineStr"><is><t>Sports Sport</t></is></c><c r="D1" t="inlineStr"><is><t>Schools Name</t></is></c><c r="E1" t="inlineStr"><is><t>Address Mailing / Permanent City</t></is></c><c r="F1" t="inlineStr"><is><t>Address Mailing / Permanent Region</t></is></c></row>
<row r="2"><c r="A2" t="inlineStr"><is><t>Alice</t></is></c><c r="B2" t="inlineStr"><is><t>Smith</t></is></c><c r="C2" t="inlineStr"><is><t>Track</t></is></c><c r="D2" t="inlineStr"><is><t>High School</t></is></c><c r="E2" t="inlineStr"><is><t>City</t></is></c><c r="F2" t="inlineStr"><is><t>State</t></is></c></row>"#;

    let sheet2_content = r#"<row r="1"><c r="A1" t="inlineStr"><is><t>Person First</t></is></c><c r="B1" t="inlineStr"><is><t>Person Last</t></is></c><c r="C1" t="inlineStr"><is><t>Sports Sport</t></is></c><c r="D1" t="inlineStr"><is><t>Schools Name</t></is></c><c r="E1" t="inlineStr"><is><t>Address Mailing / Permanent City</t></is></c><c r="F1" t="inlineStr"><is><t>Address Mailing / Permanent Region</t></is></c></row>
<row r="2"><c r="A2" t="inlineStr"><is><t>Charlie</t></is></c><c r="B2" t="inlineStr"><is><t>Davis</t></is></c><c r="C2" t="inlineStr"><is><t>Swim</t></is></c><c r="D2" t="inlineStr"><is><t>Davis High</t></is></c><c r="E2" t="inlineStr"><is><t>Davis City</t></is></c><c r="F2" t="inlineStr"><is><t>Davis State</t></is></c></row>"#;

    let generated_sheet_content = r#"<row r="1"><c r="A1" t="inlineStr"><is><t>Person First</t></is></c><c r="B1" t="inlineStr"><is><t>Person Last</t></is></c><c r="C1" t="inlineStr"><is><t>Sports Sport</t></is></c><c r="D1" t="inlineStr"><is><t>Schools Name</t></is></c><c r="E1" t="inlineStr"><is><t>Address Mailing / Permanent City</t></is></c><c r="F1" t="inlineStr"><is><t>Address Mailing / Permanent Region</t></is></c></row>
<row r="2"><c r="A2" t="inlineStr"><is><t>Bob</t></is></c><c r="B2" t="inlineStr"><is><t>Generated</t></is></c><c r="C2" t="inlineStr"><is><t>Track</t></is></c><c r="D2" t="inlineStr"><is><t>Generated School</t></is></c><c r="E2" t="inlineStr"><is><t>Generated City</t></is></c><c r="F2" t="inlineStr"><is><t>Generated State</t></is></c></row>"#;

    let sheets = vec![
        ("Export", sheet1_content),
        ("Sheet1", sheet2_content),
        ("Athletic Matches", generated_sheet_content),
    ];
    let path = build_minimal_xlsx(&dir, "fixture.xlsx", &sheets)?;

    let result = scan(&path, ScanMode::Exhaustive, None)?;

    assert_eq!(result.stats.actual_data_rows, 2);
    assert_eq!(result.prospects.len(), 2);

    let names: Vec<&str> = result
        .prospects
        .iter()
        .map(|p| p.first_name.as_str())
        .collect();
    assert!(names.contains(&"Alice"));
    assert!(names.contains(&"Charlie"));
    assert!(!names.contains(&"Bob"));

    Ok(())
}

#[test]
fn first_worksheet_generated_sheet_returns_error() -> Result<()> {
    let dir = tempdir()?;
    let sheets = vec![(
        "Athletic Matches",
        "<row r=\"1\"><c r=\"A1\" t=\"inlineStr\"><is><t>Person First</t></is></c></row>",
    )];
    let path = build_minimal_xlsx(&dir, "fixture.xlsx", &sheets)?;

    assert!(scan(&path, ScanMode::FirstWorksheet, None).is_err());

    Ok(())
}

#[test]
fn first_worksheet_missing_required_headers_returns_error() -> Result<()> {
    let dir = tempdir()?;
    let sheet1_content = r#"<row r="1"><c r="A1" t="inlineStr"><is><t>Other Header</t></is></c></row>
<row r="2"><c r="A2" t="inlineStr"><is><t>Data</t></is></c></row>"#;

    let sheets = vec![("Export", sheet1_content)];
    let path = build_minimal_xlsx(&dir, "fixture.xlsx", &sheets)?;

    assert!(scan(&path, ScanMode::FirstWorksheet, None).is_err());

    Ok(())
}

#[test]
fn first_worksheet_preserves_sparse_rich_shared_and_inline_source_rows() -> Result<()> {
    let dir = tempdir()?;
    let header_cells = crate::model::SOURCE_HEADERS
        .iter()
        .enumerate()
        .map(|(index, header)| {
            let column = column_name(index);
            format!(r#"<c r="{column}1" t="inlineStr"><is><t>{header}</t></is></c>"#)
        })
        .collect::<String>();
    let row = r#"<row r="7"><c r="A7" t="inlineStr"><is><r><t xml:space="preserve"> Alice </t></r><r><t>Runner</t></r></is></c><c r="B7" t="s"><v>0</v></c><c r="I7" t="inlineStr"><is><t xml:space="preserve"> Basketball </t></is></c><c r="M7" t="inlineStr"><is><t>Sparse School</t></is></c></row>"#;
    let sheet = format!(r#"<row r="1">{header_cells}</row>{row}"#);
    let path = build_minimal_xlsx(&dir, "fixture.xlsx", &[("Roster", &sheet)])?;

    let result = scan(&path, ScanMode::FirstWorksheet, None)?;

    assert_eq!(result.stats.actual_data_rows, 1);
    assert_eq!(result.prospects.len(), 1);
    let prospect = &result.prospects[0];
    assert_eq!(prospect.excel_row, 7);
    assert_eq!(prospect.first_name, " Alice Runner");
    assert_eq!(prospect.last_name, "Shared Family");
    assert_eq!(prospect.sport, " Basketball ");
    assert_eq!(prospect.school, "Sparse School");
    assert_eq!(
        prospect
            .source_fields
            .get("Person Email")
            .map(String::as_str),
        Some("")
    );
    Ok(())
}

#[test]
fn styled_and_explicitly_empty_rows_are_not_real_data() -> Result<()> {
    let dir = tempdir()?;
    let header_cells = crate::model::SOURCE_HEADERS
        .iter()
        .enumerate()
        .map(|(index, header)| {
            let column = column_name(index);
            format!(r#"<c r="{column}2" t="inlineStr"><is><t>{header}</t></is></c>"#)
        })
        .collect::<String>();
    let sheet = format!(
        r#"<row r="1"><c r="A1" s="1"/><c r="M1" s="1"/></row>
<row r="2">{header_cells}</row>
<row r="3"><c r="A3" s="1"/><c r="B3"><v></v></c></row>
<row r="4"><c r="A4" t="inlineStr"><is><t>Alice</t></is></c><c r="B4" t="inlineStr"><is><t>Smith</t></is></c></row>"#
    );
    let path = build_minimal_xlsx(&dir, "fixture.xlsx", &[("Roster", &sheet)])?;

    let result = scan(&path, ScanMode::FirstWorksheet, None)?;

    assert_eq!(result.stats.sheets[0].xml_rows, 4);
    assert_eq!(result.stats.sheets[0].actual_data_rows, 1);
    assert_eq!(result.prospects.len(), 1);
    assert_eq!(result.prospects[0].excel_row, 4);
    assert_eq!(result.prospects[0].first_name, "Alice");
    Ok(())
}
