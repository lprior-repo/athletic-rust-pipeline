use crate::model::{MatchRecord, Prospect, SheetStats, SourceRecord, WorkbookStats};
use anyhow::{bail, Context, Result};
use quick_xml::{events::Event, Reader};
use std::{
    collections::{BTreeMap, HashMap},
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

const REL_WORKSHEET: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet";
const CONTENT_WORKSHEET: &str =
    "application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml";

#[derive(Debug)]
pub struct ScanResult {
    pub stats: WorkbookStats,
    pub prospects: Vec<Prospect>,
}

#[derive(Debug, Clone)]
struct SheetMeta {
    name: String,
    path: String,
    sheet_id: u32,
}

#[derive(Debug, Default)]
struct CellState {
    reference: String,
    cell_type: String,
    value: String,
}

pub fn scan(
    path: &Path,
    target_sports: &[String],
    expected_graduation_year: Option<i32>,
) -> Result<ScanResult> {
    let shared_strings = load_shared_strings(path)?;
    let sheets = load_sheet_metadata(path)?;
    let normalized_sports: Vec<String> = target_sports.iter().map(|x| normalize(x)).collect();
    let mut stats = WorkbookStats::default();
    let mut prospects = Vec::new();

    for sheet in sheets {
        let mut archive = ZipArchive::new(File::open(path)?)?;
        let file = archive
            .by_name(&sheet.path)
            .with_context(|| format!("opening {} in workbook", sheet.path))?;
        let sheet_name = sheet.name.clone();
        let mut selected_in_sheet = 0_u64;
        let sheet_stats = parse_worksheet(file, &sheet_name, &shared_strings, |record| {
            if normalized_sports.is_empty() {
                return Ok(());
            }
            let sport = field(&record, "Sports Sport");
            if !normalized_sports
                .iter()
                .any(|target| normalize(sport) == *target)
            {
                return Ok(());
            }
            let first_name = field(&record, "Person First").to_owned();
            let last_name = field(&record, "Person Last").to_owned();
            if first_name.trim().is_empty() && last_name.trim().is_empty() {
                return Ok(());
            }
            prospects.push(Prospect {
                source_key: record.source_key.clone(),
                sheet: record.sheet.clone(),
                excel_row: record.excel_row,
                first_name,
                last_name,
                school: field(&record, "Schools Name").to_owned(),
                city: field(&record, "Address Mailing / Permanent City").to_owned(),
                state: field(&record, "Address Mailing / Permanent Region").to_owned(),
                sport: sport.to_owned(),
                expected_graduation_year,
            });
            selected_in_sheet = selected_in_sheet.saturating_add(1);
            Ok(())
        })?;
        stats.actual_data_rows = stats
            .actual_data_rows
            .saturating_add(sheet_stats.actual_data_rows);
        stats.selected_prospects = stats.selected_prospects.saturating_add(selected_in_sheet);
        stats.sheets.push(sheet_stats);
    }

    Ok(ScanResult { stats, prospects })
}

pub fn export_records(input: &Path, output: &Path) -> Result<()> {
    if output.exists() {
        bail!("refusing to overwrite existing {}", output.display());
    }
    let shared_strings = load_shared_strings(input)?;
    let sheets = load_sheet_metadata(input)?;
    let mut writer = BufWriter::new(
        File::create(output).with_context(|| format!("creating {}", output.display()))?,
    );
    for sheet in sheets {
        let mut archive = ZipArchive::new(File::open(input)?)?;
        let file = archive.by_name(&sheet.path)?;
        parse_worksheet(file, &sheet.name, &shared_strings, |record| {
            serde_json::to_writer(&mut writer, &record)?;
            writer.write_all(b"\n")?;
            Ok(())
        })?;
    }
    writer.flush()?;
    Ok(())
}

fn field<'a>(record: &'a SourceRecord, header: &str) -> &'a str {
    record.fields.get(header).map_or("", String::as_str)
}

fn normalize(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn load_shared_strings(path: &Path) -> Result<Vec<String>> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let file = match archive.by_name("xl/sharedStrings.xml") {
        Ok(file) => file,
        Err(zip::result::ZipError::FileNotFound) => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.config_mut().trim_text(false);
    let mut buffer = Vec::new();
    let mut strings = Vec::new();
    let mut in_item = false;
    let mut in_text = false;
    let mut current = String::new();

    loop {
        match reader.read_event_into(&mut buffer)? {
            Event::Start(event) if event.name().as_ref() == b"si" => {
                in_item = true;
                current.clear();
            }
            Event::Start(event) if in_item && event.name().as_ref() == b"t" => in_text = true,
            Event::Text(event) if in_item && in_text => {
                current.push_str(&decode_xml_text(event.as_ref())?);
            }
            Event::End(event) if event.name().as_ref() == b"t" => in_text = false,
            Event::End(event) if event.name().as_ref() == b"si" => {
                strings.push(std::mem::take(&mut current));
                in_item = false;
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    Ok(strings)
}

fn load_sheet_metadata(path: &Path) -> Result<Vec<SheetMeta>> {
    let mut archive = ZipArchive::new(File::open(path)?)?;
    let workbook_xml = read_zip_string(&mut archive, "xl/workbook.xml")?;
    let relationships_xml = read_zip_string(&mut archive, "xl/_rels/workbook.xml.rels")?;
    let relationships = parse_relationships(&relationships_xml)?;
    let mut reader = Reader::from_str(&workbook_xml);
    let mut buffer = Vec::new();
    let mut sheets = Vec::new();

    loop {
        match reader.read_event_into(&mut buffer)? {
            Event::Empty(event) | Event::Start(event) if event.name().as_ref() == b"sheet" => {
                let name = attribute(&event, b"name")?.unwrap_or_else(String::new);
                let sheet_id = attribute(&event, b"sheetId")?
                    .and_then(|value| value.parse().ok())
                    .map_or(0, |value| value);
                let relation_id = event
                    .attributes()
                    .with_checks(false)
                    .filter_map(|item| item.ok())
                    .find(|item| {
                        item.key.as_ref() == b"r:id" || item.key.as_ref().ends_with(b":id")
                    })
                    .map(|item| decode_xml_text(item.value.as_ref()))
                    .transpose()?
                    .unwrap_or_else(String::new);
                let target = relationships.get(&relation_id).with_context(|| {
                    format!("missing worksheet relationship {relation_id} for {name}")
                })?;
                sheets.push(SheetMeta {
                    name,
                    path: normalize_zip_target(target),
                    sheet_id,
                });
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    Ok(sheets)
}

fn parse_relationships(xml: &str) -> Result<HashMap<String, String>> {
    let mut reader = Reader::from_str(xml);
    let mut buffer = Vec::new();
    let mut relationships = HashMap::new();
    loop {
        match reader.read_event_into(&mut buffer)? {
            Event::Empty(event) | Event::Start(event)
                if event.name().as_ref() == b"Relationship" =>
            {
                if let (Some(id), Some(target)) =
                    (attribute(&event, b"Id")?, attribute(&event, b"Target")?)
                {
                    relationships.insert(id, target);
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
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

fn parse_worksheet<R, F>(
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
    let mut stats = SheetStats {
        name: sheet_name.to_owned(),
        ..Default::default()
    };
    let mut headers: Vec<String> = Vec::new();
    let mut header_row = None;
    let mut current_row_number = 0_u32;
    let mut current_cells: BTreeMap<usize, String> = BTreeMap::new();
    let mut current_cell: Option<CellState> = None;
    let mut in_value = false;
    let mut in_inline_text = false;

    loop {
        match reader.read_event_into(&mut buffer)? {
            Event::Empty(event) if event.name().as_ref() == b"dimension" => {
                stats.declared_dimension = attribute(&event, b"ref")?;
            }
            Event::Start(event) if event.name().as_ref() == b"row" => {
                stats.xml_rows = stats.xml_rows.saturating_add(1);
                let fallback_row = u32::try_from(stats.xml_rows).map_or(u32::MAX, |value| value);
                current_row_number = attribute(&event, b"r")?
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(fallback_row);
                current_cells.clear();
            }
            Event::Start(event) if event.name().as_ref() == b"c" => {
                current_cell = Some(CellState {
                    reference: attribute(&event, b"r")?.unwrap_or_else(String::new),
                    cell_type: attribute(&event, b"t")?.unwrap_or_else(String::new),
                    value: String::new(),
                });
            }
            Event::Start(event) if event.name().as_ref() == b"v" => in_value = true,
            Event::Start(event) if event.name().as_ref() == b"t" => in_inline_text = true,
            Event::Text(event) if in_value || in_inline_text => {
                if let Some(cell) = current_cell.as_mut() {
                    cell.value.push_str(&decode_xml_text(event.as_ref())?);
                }
            }
            Event::End(event) if event.name().as_ref() == b"v" => in_value = false,
            Event::End(event) if event.name().as_ref() == b"t" => in_inline_text = false,
            Event::End(event) if event.name().as_ref() == b"c" => {
                if let Some(cell) = current_cell.take() {
                    let column = column_index(&cell.reference)?;
                    let value = resolve_cell_value(&cell, shared_strings);
                    if !value.is_empty() {
                        current_cells.insert(column, value);
                    }
                }
            }
            Event::End(event) if event.name().as_ref() == b"row" => {
                if current_cells.is_empty() {
                    buffer.clear();
                    continue;
                }
                if header_row.is_none() {
                    let max_column = current_cells.keys().copied().max().unwrap_or(0);
                    headers = (0..=max_column)
                        .map(|column| current_cells.get(&column).cloned().unwrap_or_default())
                        .collect();
                    header_row = Some(current_row_number);
                    stats.headers = headers.clone();
                } else {
                    stats.actual_data_rows = stats.actual_data_rows.saturating_add(1);
                    stats.last_actual_row = current_row_number;
                    let fields = headers
                        .iter()
                        .enumerate()
                        .filter(|(_, header)| !header.is_empty())
                        .map(|(column, header)| {
                            (
                                header.clone(),
                                current_cells.get(&column).cloned().unwrap_or_default(),
                            )
                        })
                        .collect();
                    on_record(SourceRecord {
                        source_key: format!("{sheet_name}:{current_row_number}"),
                        sheet: sheet_name.to_owned(),
                        excel_row: current_row_number,
                        fields,
                    })?;
                }
            }
            Event::Eof => break,
            _ => {}
        }
        buffer.clear();
    }
    Ok(stats)
}

fn resolve_cell_value(cell: &CellState, shared_strings: &[String]) -> String {
    if cell.cell_type == "s" {
        cell.value
            .parse::<usize>()
            .ok()
            .and_then(|index| shared_strings.get(index))
            .cloned()
            .unwrap_or_default()
    } else {
        cell.value.trim().to_owned()
    }
}

fn column_index(reference: &str) -> Result<usize> {
    let mut result = 0_usize;
    let mut letters = 0_usize;
    for byte in reference.bytes() {
        if !byte.is_ascii_alphabetic() {
            break;
        }
        let Some(offset) = byte.to_ascii_uppercase().checked_sub(b'A') else {
            bail!("invalid cell reference {reference:?}");
        };
        let Some(offset) = offset.checked_add(1) else {
            bail!("invalid cell reference {reference:?}");
        };
        result = result
            .checked_mul(26)
            .and_then(|value| value.checked_add(usize::from(offset)))
            .with_context(|| format!("cell reference column overflow: {reference:?}"))?;
        letters = letters.saturating_add(1);
    }
    if letters == 0 {
        bail!("invalid cell reference {reference:?}");
    }
    result
        .checked_sub(1)
        .with_context(|| format!("invalid cell reference {reference:?}"))
}

fn attribute(event: &quick_xml::events::BytesStart<'_>, key: &[u8]) -> Result<Option<String>> {
    for attribute in event.attributes().with_checks(false) {
        let attribute = attribute?;
        if attribute.key.as_ref() == key {
            return Ok(Some(decode_xml_text(attribute.value.as_ref())?));
        }
    }
    Ok(None)
}

fn decode_xml_text(value: &[u8]) -> Result<String> {
    let text = std::str::from_utf8(value)?;
    Ok(quick_xml::escape::unescape(text)?.into_owned())
}

fn read_zip_string<R: Read + std::io::Seek>(
    archive: &mut ZipArchive<R>,
    name: &str,
) -> Result<String> {
    let mut value = String::new();
    archive.by_name(name)?.read_to_string(&mut value)?;
    Ok(value)
}

pub fn append_matches_sheet(input: &Path, output: &Path, records: &[MatchRecord]) -> Result<()> {
    if input == output {
        bail!("source and output paths must be different");
    }
    if output.exists() {
        bail!("refusing to overwrite existing {}", output.display());
    }

    let sheets = load_sheet_metadata(input)?;
    if sheets.iter().any(|sheet| sheet.name == "Athletic Matches") {
        bail!("source workbook already has an Athletic Matches worksheet");
    }
    let maximum_sheet_id = sheets
        .iter()
        .map(|sheet| sheet.sheet_id)
        .max()
        .map_or(0, |id| id);
    let next_sheet_id = maximum_sheet_id
        .checked_add(1)
        .context("worksheet id overflow")?;
    let next_sheet_path = format!("xl/worksheets/sheet{next_sheet_id}.xml");

    let mut source = ZipArchive::new(File::open(input)?)?;
    let workbook_xml = read_zip_string(&mut source, "xl/workbook.xml")?;
    let relationships_xml = read_zip_string(&mut source, "xl/_rels/workbook.xml.rels")?;
    let content_types_xml = read_zip_string(&mut source, "[Content_Types].xml")?;
    let next_relation_id = next_relationship_id(&relationships_xml)?;
    drop(source);

    let workbook_xml = insert_before(
        &workbook_xml,
        "</sheets>",
        &format!(
            "<sheet name=\"Athletic Matches\" sheetId=\"{next_sheet_id}\" r:id=\"{next_relation_id}\"/>"
        ),
    )?;
    let relationships_xml = insert_before(
        &relationships_xml,
        "</Relationships>",
        &format!(
            "<Relationship Id=\"{next_relation_id}\" Type=\"{REL_WORKSHEET}\" Target=\"worksheets/sheet{next_sheet_id}.xml\"/>"
        ),
    )?;
    let content_types_xml = insert_before(
        &content_types_xml,
        "</Types>",
        &format!(
            "<Override PartName=\"/xl/worksheets/sheet{next_sheet_id}.xml\" ContentType=\"{CONTENT_WORKSHEET}\"/>"
        ),
    )?;
    let result_sheet = build_matches_sheet(records);

    let temporary = temporary_output_path(output);
    let mut source = ZipArchive::new(File::open(input)?)?;
    let destination = File::create(&temporary)?;
    let mut destination = ZipWriter::new(BufWriter::new(destination));
    let replacements: HashMap<&str, &[u8]> = HashMap::from([
        ("xl/workbook.xml", workbook_xml.as_bytes()),
        ("xl/_rels/workbook.xml.rels", relationships_xml.as_bytes()),
        ("[Content_Types].xml", content_types_xml.as_bytes()),
    ]);

    for index in 0..source.len() {
        let mut file = source.by_index(index)?;
        let name = file.name().to_owned();
        let options = SimpleFileOptions::default().compression_method(file.compression());
        destination.start_file(&name, options)?;
        if let Some(replacement) = replacements.get(name.as_str()) {
            destination.write_all(replacement)?;
        } else {
            std::io::copy(&mut file, &mut destination)?;
        }
    }
    destination.start_file(
        &next_sheet_path,
        SimpleFileOptions::default().compression_method(CompressionMethod::Deflated),
    )?;
    destination.write_all(result_sheet.as_bytes())?;
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

fn next_relationship_id(xml: &str) -> Result<String> {
    let relationships = parse_relationships(xml)?;
    let maximum = relationships
        .keys()
        .filter_map(|id| id.strip_prefix("rId")?.parse::<u32>().ok())
        .max()
        .map_or(0, |id| id);
    let next_id = maximum.checked_add(1).context("relationship id overflow")?;
    Ok(format!("rId{next_id}"))
}

fn insert_before(source: &str, marker: &str, insertion: &str) -> Result<String> {
    let index = source
        .rfind(marker)
        .with_context(|| format!("missing XML marker {marker}"))?;
    let prefix = source
        .get(..index)
        .with_context(|| format!("invalid XML marker boundary {marker}"))?;
    let suffix = source
        .get(index..)
        .with_context(|| format!("invalid XML marker boundary {marker}"))?;
    let mut output = String::with_capacity(source.len().saturating_add(insertion.len()));
    output.push_str(prefix);
    output.push_str(insertion);
    output.push_str(suffix);
    Ok(output)
}

fn temporary_output_path(output: &Path) -> PathBuf {
    let mut value = output.as_os_str().to_owned();
    value.push(format!(".{}.tmp", std::process::id()));
    PathBuf::from(value)
}

fn build_matches_sheet(records: &[MatchRecord]) -> String {
    let headers = [
        "Source Key",
        "Source Sheet",
        "Excel Row",
        "Prospect Name",
        "Prospect School",
        "Prospect Sport",
        "Status",
        "Score",
        "Matched Name",
        "Matched School",
        "Matched Location",
        "Profile",
        "Track Confirmed",
        "XC Confirmed",
        "100m PR",
        "200m PR",
        "400m PR",
        "800m PR",
        "1600m PR",
        "3200m PR",
        "Hurdles PR",
        "High Jump PR",
        "Long Jump PR",
        "Triple Jump PR",
        "Pole Vault PR",
        "Shot Put PR",
        "Discus PR",
        "Javelin PR",
        "Candidates JSON",
        "Best Marks JSON",
        "Notes",
        "Hint Count",
        "AI Logic",
    ];
    let final_column = column_name(headers.len().saturating_sub(1));
    let final_row = records.len().saturating_add(1);
    let mut xml = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><dimension ref=\"A1:{final_column}{final_row}\"/><sheetViews><sheetView workbookViewId=\"0\"/></sheetViews><sheetFormatPr defaultRowHeight=\"15\"/><sheetData>"
    );
    xml.push_str("<row r=\"1\">");
    for (column, header) in headers.iter().enumerate() {
        push_inline_cell(&mut xml, 1, column, header);
    }
    xml.push_str("</row>");

    for (offset, record) in records.iter().enumerate() {
        let row = offset.saturating_add(2);
        xml.push_str(&format!("<row r=\"{row}\">"));
        let values = [
            record.source_key.clone(),
            record.prospect.sheet.clone(),
            record.prospect.excel_row.to_string(),
            record.prospect.full_name(),
            record.prospect.school.clone(),
            record.prospect.sport.clone(),
            record.status.clone(),
            format!("{:.4}", record.score),
            record.selected_name.clone(),
            record.selected_school.clone(),
            record.selected_location.clone(),
            String::new(),
            yes_no(record.track_confirmed).to_owned(),
            yes_no(record.xc_confirmed).to_owned(),
            mark_value(record, "100m"),
            mark_value(record, "200m"),
            mark_value(record, "400m"),
            mark_value(record, "800m"),
            mark_value(record, "1600m"),
            mark_value(record, "3200m"),
            first_mark(record, &["100h", "110h", "300h", "400h"]),
            mark_value(record, "high_jump"),
            mark_value(record, "long_jump"),
            mark_value(record, "triple_jump"),
            mark_value(record, "pole_vault"),
            mark_value(record, "shot_put"),
            mark_value(record, "discus"),
            mark_value(record, "javelin"),
            serde_json::to_string(&record.candidates).unwrap_or_else(|_| String::new()),
            serde_json::to_string(&record.best_marks).unwrap_or_else(|_| String::new()),
            record.notes.clone(),
            record.hint_count.to_string(),
            record.ai_logic.clone(),
        ];
        for (column, value) in values.iter().enumerate() {
            if column == 7 {
                push_number_cell(&mut xml, row, column, value);
            } else if column == 11 && !record.selected_profile_url.is_empty() {
                push_hyperlink_formula_cell(&mut xml, row, column, &record.selected_profile_url);
            } else {
                push_inline_cell(&mut xml, row, column, value);
            }
        }
        xml.push_str("</row>");
    }
    xml.push_str(&format!(
        "</sheetData><autoFilter ref=\"A1:{final_column}{final_row}\"/></worksheet>"
    ));
    xml
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "YES"
    } else {
        "NO"
    }
}

fn mark_value(record: &MatchRecord, event: &str) -> String {
    record
        .best_marks
        .get(event)
        .map_or_else(String::new, |mark| mark.mark.clone())
}

fn first_mark(record: &MatchRecord, events: &[&str]) -> String {
    events
        .iter()
        .find_map(|event| record.best_marks.get(*event).map(|mark| mark.mark.clone()))
        .unwrap_or_else(String::new)
}

fn push_inline_cell(xml: &mut String, row: usize, column: usize, value: &str) {
    let reference = format!("{}{}", column_name(column), row);
    xml.push_str(&format!(
        "<c r=\"{reference}\" t=\"inlineStr\"><is><t xml:space=\"preserve\">{}</t></is></c>",
        escape_xml(value)
    ));
}

fn push_number_cell(xml: &mut String, row: usize, column: usize, value: &str) {
    let reference = format!("{}{}", column_name(column), row);
    xml.push_str(&format!("<c r=\"{reference}\"><v>{value}</v></c>"));
}

fn push_hyperlink_formula_cell(xml: &mut String, row: usize, column: usize, url: &str) {
    let reference = format!("{}{}", column_name(column), row);
    let formula_url = url.replace('"', "\"\"");
    let formula = format!("HYPERLINK(\"{formula_url}\",\"Open profile\")");
    xml.push_str(&format!(
        "<c r=\"{reference}\"><f>{}</f><v></v></c>",
        escape_xml(&formula)
    ));
}

fn column_name(mut index: usize) -> String {
    let mut output = Vec::new();
    index = index.saturating_add(1);
    while index > 0 {
        let Some(adjusted) = index.checked_sub(1) else {
            return String::new();
        };
        let remainder = adjusted % 26;
        let Ok(offset) = u8::try_from(remainder) else {
            return String::new();
        };
        let Some(byte) = b'A'.checked_add(offset) else {
            return String::new();
        };
        output.push(char::from(byte));
        index = adjusted / 26;
    }
    output.iter().rev().collect()
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn converts_column_references() {
        assert_eq!(column_index("A2").unwrap(), 0);
        assert_eq!(column_index("Z2").unwrap(), 25);
        assert_eq!(column_index("AA2").unwrap(), 26);
        assert_eq!(column_name(0), "A");
        assert_eq!(column_name(28), "AC");
    }

    #[test]
    fn escapes_xml() {
        assert_eq!(escape_xml("A&B <C>"), "A&amp;B &lt;C&gt;");
    }

    #[test]
    fn streams_real_rows_and_ignores_styled_empty_rows() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("fixture.xlsx");
        let file = File::create(&path).unwrap();
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        write_fixture_entry(
            &mut zip,
            "[Content_Types].xml",
            r#"<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>"#,
            options,
        );
        write_fixture_entry(
            &mut zip,
            "xl/workbook.xml",
            r#"<?xml version="1.0" encoding="UTF-8"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Export" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
            options,
        );
        write_fixture_entry(
            &mut zip,
            "xl/_rels/workbook.xml.rels",
            r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
            options,
        );
        write_fixture_entry(
            &mut zip,
            "xl/worksheets/sheet1.xml",
            r#"<?xml version="1.0" encoding="UTF-8"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><dimension ref="A1:F1000"/><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Person First</t></is></c><c r="B1" t="inlineStr"><is><t>Person Last</t></is></c><c r="C1" t="inlineStr"><is><t>Sports Sport</t></is></c><c r="D1" t="inlineStr"><is><t>Schools Name</t></is></c><c r="E1" t="inlineStr"><is><t>Address Mailing / Permanent City</t></is></c><c r="F1" t="inlineStr"><is><t>Address Mailing / Permanent Region</t></is></c></row><row r="2"><c r="A2" t="inlineStr"><is><t>Sarah</t></is></c><c r="B2" t="inlineStr"><is><t>Jones</t></is></c><c r="C2" t="inlineStr"><is><t>Women's Track &amp; Field</t></is></c><c r="D2" t="inlineStr"><is><t>Central High</t></is></c><c r="E2" t="inlineStr"><is><t>Austin</t></is></c><c r="F2" t="inlineStr"><is><t>TX</t></is></c></row><row r="1000"><c r="A1000" s="1"/></row></sheetData></worksheet>"#,
            options,
        );
        zip.finish().unwrap();

        let result = scan(&path, &["Women's Track & Field".to_owned()], Some(2027)).unwrap();
        assert_eq!(result.stats.actual_data_rows, 1);
        assert_eq!(result.stats.sheets[0].xml_rows, 3);
        assert_eq!(result.stats.sheets[0].last_actual_row, 2);
        assert_eq!(result.prospects.len(), 1);
        assert_eq!(result.prospects[0].source_key, "Export:2");

        let enriched = directory.path().join("enriched.xlsx");
        let record = MatchRecord {
            source_key: "Export:2".to_owned(),
            prospect: result.prospects[0].clone(),
            status: "MATCH".to_owned(),
            score: 0.97,
            selected_profile_url: "https://www.athletic.net/athlete/123/track-and-field".to_owned(),
            selected_name: "Sarah Jones".to_owned(),
            ..Default::default()
        };
        append_matches_sheet(&path, &enriched, &[record]).unwrap();
        let mut archive = ZipArchive::new(File::open(&enriched).unwrap()).unwrap();
        let workbook = read_zip_string(&mut archive, "xl/workbook.xml").unwrap();
        let result_sheet = read_zip_string(&mut archive, "xl/worksheets/sheet2.xml").unwrap();
        assert!(workbook.contains("Athletic Matches"));
        assert!(result_sheet.contains("HYPERLINK"));
        assert!(result_sheet.contains("athlete/123/track-and-field"));
        assert!(result_sheet.contains("Candidates JSON"));
        assert!(result_sheet.contains("Best Marks JSON"));
        assert!(result_sheet.contains("Hint Count"));
        assert!(result_sheet.contains("AI Logic"));
    }

    fn write_fixture_entry<W: Write + std::io::Seek>(
        zip: &mut ZipWriter<W>,
        name: &str,
        contents: &str,
        options: SimpleFileOptions,
    ) {
        zip.start_file(name, options).unwrap();
        zip.write_all(contents.as_bytes()).unwrap();
    }
}
