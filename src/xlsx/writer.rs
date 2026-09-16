use super::metadata::{bounded_entry, load_sheet_metadata, read_zip_string};
use super::writer_xml::{
    build_matches_sheet, insert_before, next_relationship_id, temporary_output_path,
};
use super::{CONTENT_WORKSHEET, REL_WORKSHEET};
use crate::model::{MatchRecord, SourceRecord};
use anyhow::{bail, Context, Result};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

pub fn export_records(input: &Path, output: &Path) -> Result<()> {
    if output.exists() {
        bail!("refusing to overwrite existing {}", output.display());
    }
    let mut writer = BufWriter::new(
        File::create(output).with_context(|| format!("creating {}", output.display()))?,
    );
    super::walk_records(input, &super::ScanMode::Exhaustive, |record| {
        write_record(&mut writer, record)
    })?;
    writer.flush()?;
    Ok(())
}

fn write_record(writer: &mut BufWriter<File>, record: SourceRecord) -> Result<()> {
    serde_json::to_writer(&mut *writer, &record)?;
    writer.write_all(b"\n")?;
    Ok(())
}

pub fn append_matches_sheet(input: &Path, output: &Path, records: &[MatchRecord]) -> Result<()> {
    validate_paths(input, output)?;
    let sheets = load_sheet_metadata(input)?;
    let existing_path = sheets
        .iter()
        .find(|sheet| sheet.name == "Athletic Matches")
        .map(|sheet| sheet.path.clone());
    let (result_sheet_path, workbook_xml, relationships_xml, content_types_xml, replacing) =
        output_metadata(input, &sheets, existing_path)?;
    let result_sheet = build_matches_sheet(records)?;
    let temporary = temporary_output_path(output);
    let mut source = ZipArchive::new(File::open(input)?)?;
    let destination = File::create(&temporary)?;
    let mut destination = ZipWriter::new(BufWriter::new(destination));
    let replacements: HashMap<&str, &[u8]> = HashMap::from([
        ("xl/workbook.xml", workbook_xml.as_bytes()),
        ("xl/_rels/workbook.xml.rels", relationships_xml.as_bytes()),
        ("[Content_Types].xml", content_types_xml.as_bytes()),
    ]);
    (0..source.len()).try_for_each(|index| {
        let file = source.by_index(index)?;
        let name = file.name().to_owned();
        let options = SimpleFileOptions::default().compression_method(file.compression());
        destination.start_file(&name, options)?;
        if name == result_sheet_path {
            destination.write_all(result_sheet.as_bytes())?;
        } else if let Some(replacement) = replacements.get(name.as_str()) {
            destination.write_all(replacement)?;
        } else {
            let mut bounded = bounded_entry(file)?;
            std::io::copy(&mut bounded, &mut destination)?;
        }
        Ok::<(), anyhow::Error>(())
    })?;
    if !replacing {
        destination.start_file(
            &result_sheet_path,
            SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
        )?;
        destination.write_all(result_sheet.as_bytes())?;
    }
    destination.finish()?.flush()?;
    fs::rename(&temporary, output).with_context(|| {
        format!(
            "promoting temporary workbook {} to {}",
            temporary.display(),
            output.display()
        )
    })?;
    Ok(())
}

fn validate_paths(input: &Path, output: &Path) -> Result<()> {
    if input == output {
        bail!("source and output paths must be different");
    }
    if output.exists() {
        bail!("refusing to overwrite existing {}", output.display());
    }
    Ok(())
}

fn output_metadata(
    input: &Path,
    sheets: &[super::SheetMeta],
    existing_path: Option<String>,
) -> Result<(String, String, String, String, bool)> {
    let mut source = ZipArchive::new(File::open(input)?)?;
    let workbook_xml = read_zip_string(&mut source, "xl/workbook.xml")?;
    let relationships_xml = read_zip_string(&mut source, "xl/_rels/workbook.xml.rels")?;
    let content_types_xml = read_zip_string(&mut source, "[Content_Types].xml")?;
    match existing_path {
        Some(path) => Ok((
            path,
            workbook_xml,
            relationships_xml,
            content_types_xml,
            true,
        )),
        None => create_output_metadata(sheets, workbook_xml, relationships_xml, content_types_xml),
    }
}

fn create_output_metadata(
    sheets: &[super::SheetMeta],
    workbook: String,
    relationships: String,
    content_types: String,
) -> Result<(String, String, String, String, bool)> {
    let maximum = sheets.iter().map(|sheet| sheet.sheet_id).fold(0, u32::max);
    let next_sheet_id = maximum.checked_add(1).context("worksheet id overflow")?;
    let path = format!("xl/worksheets/sheet{next_sheet_id}.xml");
    let next_relation_id = next_relationship_id(&relationships)?;
    let workbook = insert_before(&workbook, "</sheets>", &format!("<sheet name=\"Athletic Matches\" sheetId=\"{next_sheet_id}\" r:id=\"{next_relation_id}\"/>"))?;
    let relationships = insert_before(&relationships, "</Relationships>", &format!("<Relationship Id=\"{next_relation_id}\" Type=\"{REL_WORKSHEET}\" Target=\"worksheets/sheet{next_sheet_id}.xml\"/>"))?;
    let content_types = insert_before(
        &content_types,
        "</Types>",
        &format!("<Override PartName=\"/{path}\" ContentType=\"{CONTENT_WORKSHEET}\"/>"),
    )?;
    Ok((path, workbook, relationships, content_types, false))
}
