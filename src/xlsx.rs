use crate::model::{Prospect, SourceRecord};

pub(crate) mod cells;
pub(crate) mod metadata;
mod parser;
mod selection;
mod writer;
mod writer_xml;
pub(crate) mod xml;

pub(crate) use metadata::SheetMeta;
pub use parser::visit_records;
pub(crate) use parser::walk_records;
pub use selection::{scan, ScanMode, ScanResult};
pub use writer::append_matches_sheet;
pub use writer::export_records;

#[cfg(test)]
pub(crate) use cells::{column_index, column_name, escape_xml};
#[cfg(test)]
pub(crate) use metadata::{read_zip_string, BoundedReader};
pub(crate) const MAX_ZIP_ENTRY_BYTES: u64 = 512 * 1024 * 1024;
pub(crate) const MAX_SHARED_STRINGS: usize = 2_000_000;
pub(crate) const MAX_SHARED_STRING_BYTES: usize = 4 * 1024 * 1024;
pub(crate) const MAX_SHARED_STRING_TOTAL_BYTES: usize = 256 * 1024 * 1024;
pub(crate) const MAX_WORKSHEETS: usize = 4096;
pub(crate) const MAX_EXCEL_ROW: u32 = 1_048_576;
pub(crate) const MAX_EXCEL_COLUMN: usize = 16_384;
pub(crate) const CONTENT_WORKSHEET: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";
pub(crate) const REL_WORKSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";

pub(crate) fn prospect_from_record(
    record: SourceRecord,
    expected_graduation_year: Option<i32>,
) -> Prospect {
    let SourceRecord {
        source_key,
        sheet,
        excel_row,
        fields,
    } = record;
    let first_name = fields
        .get("Person First")
        .map_or("", String::as_str)
        .to_owned();
    let last_name = fields
        .get("Person Last")
        .map_or("", String::as_str)
        .to_owned();
    let school = fields
        .get("Schools Name")
        .map_or("", String::as_str)
        .to_owned();
    let city = fields
        .get("Address Mailing / Permanent City")
        .map_or("", String::as_str)
        .to_owned();
    let state = fields
        .get("Address Mailing / Permanent Region")
        .map_or("", String::as_str)
        .to_owned();
    let sport = fields
        .get("Sports Sport")
        .map_or("", String::as_str)
        .to_owned();
    Prospect {
        source_key,
        sheet,
        excel_row,
        first_name,
        last_name,
        school,
        city,
        state,
        sport,
        expected_graduation_year,
        source_fields: fields,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn converts_column_references() -> Result<()> {
        {
            let left_value = &(column_index("A2")?);
            let right_value = &0;
            anyhow::ensure!(
                left_value == right_value,
                "left={left_value:?} right={right_value:?}"
            );
        }
        {
            let left_value = &(column_index("Z2")?);
            let right_value = &25;
            anyhow::ensure!(
                left_value == right_value,
                "left={left_value:?} right={right_value:?}"
            );
        }
        {
            let left_value = &(column_index("AA2")?);
            let right_value = &26;
            anyhow::ensure!(
                left_value == right_value,
                "left={left_value:?} right={right_value:?}"
            );
        }
        {
            let left_value = &(column_name(0));
            let right_value = &"A";
            anyhow::ensure!(
                left_value == right_value,
                "left={left_value:?} right={right_value:?}"
            );
        }
        {
            let left_value = &(column_name(28));
            let right_value = &"AC";
            anyhow::ensure!(
                left_value == right_value,
                "left={left_value:?} right={right_value:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn escapes_xml() {
        assert_eq!(escape_xml("A&B <C>"), "A&amp;B &lt;C&gt;");
    }

    #[test]
    fn streams_real_rows_and_ignores_styled_empty_rows() -> Result<()> {
        let directory = tempdir()?;
        let path = directory.path().join("fixture.xlsx");
        let file = std::fs::File::create(&path)?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        write_fixture_entry(
            &mut zip,
            "[Content_Types].xml",
            r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#,
            options,
        )?;
        write_fixture_entry(
            &mut zip,
            "xl/workbook.xml",
            r#"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Export" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
            options,
        )?;
        write_fixture_entry(
            &mut zip,
            "xl/_rels/workbook.xml.rels",
            r#"<?xml version="1.0" encoding="UTF-8"?><Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
            options,
        )?;
        write_fixture_entry(
            &mut zip,
            "xl/worksheets/sheet1.xml",
            r#"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dimension ref="A1:F1000"/><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Person First</t></is></c><c r="B1" t="inlineStr"><is><t>Person Last</t></is></c><c r="C1" t="inlineStr"><is><t>Sports Sport</t></is></c><c r="D1" t="inlineStr"><is><t>Schools Name</t></is></c><c r="E1" t="inlineStr"><is><t>Address Mailing / Permanent City</t></is></c><c r="F1" t="inlineStr"><is><t>Address Mailing / Permanent Region</t></is></c></row><row r="2"><c r="A2" t="inlineStr"><is><t>Sarah</t></is></c><c r="B2" t="inlineStr"><is><t>Jones</t></is></c><c r="C2" t="inlineStr"><is><t>Women's Track &amp; Field</t></is></c><c r="D2" t="inlineStr"><is><t>Central High</t></is></c><c r="E2" t="inlineStr"><is><t>Town</t></is></c><c r="F2" t="inlineStr"><is><t>ST</t></is></c></row><row r="3" s="1"/></sheetData></worksheet>"#,
            options,
        )?;
        zip.finish()?;
        let result = scan(
            &path,
            ScanMode::Sports(vec!["Women's Track & Field".to_owned()]),
            Some(2027),
        )?;
        anyhow::ensure!(
            result.stats.actual_data_rows == 1,
            "left={:?} right={:?}",
            &result.stats.actual_data_rows,
            &1
        );
        anyhow::ensure!(
            result.stats.sheets[0].xml_rows == 3,
            "left={:?} right={:?}",
            &result.stats.sheets[0].xml_rows,
            &3
        );
        anyhow::ensure!(
            result.stats.sheets[0].last_actual_row == 2,
            "left={:?} right={:?}",
            &result.stats.sheets[0].last_actual_row,
            &2
        );
        anyhow::ensure!(
            result.prospects.len() == 1,
            "left={:?} right={:?}",
            &result.prospects.len(),
            &1
        );
        anyhow::ensure!(
            result.prospects[0].source_key == "Export:2",
            "left={:?} right={:?}",
            &result.prospects[0].source_key,
            &"Export:2"
        );
        Ok(())
    }

    fn write_fixture_entry<W: std::io::Write + std::io::Seek>(
        zip: &mut zip::ZipWriter<W>,
        name: &str,
        contents: &str,
        options: zip::write::SimpleFileOptions,
    ) -> Result<()> {
        zip.start_file(name, options)?;
        zip.write_all(contents.as_bytes())?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "xlsx_scope_tests.rs"]
mod scope_tests;
#[cfg(test)]
#[path = "xlsx_stream_tests.rs"]
mod stream_tests;
