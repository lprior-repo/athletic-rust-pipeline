use super::cells::{column_name, escape_xml};
use super::metadata::parse_relationships;
use crate::model::{ordered_source_headers, MatchRecord};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const RESULT_HEADERS: &[&str] = &[
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

pub(crate) fn build_matches_sheet(records: &[MatchRecord]) -> Result<String> {
    let source_headers = ordered_source_headers(records);
    let mut headers = source_headers.clone();
    headers.extend(RESULT_HEADERS.iter().map(|header| (*header).to_owned()));
    let final_column = column_name(headers.len().saturating_sub(1));
    let final_row = records.len().saturating_add(1);
    let mut xml = format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?><worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><dimension ref=\"A1:{final_column}{final_row}\"/><sheetViews><sheetView workbookViewId=\"0\"/></sheetViews><sheetFormatPr defaultRowHeight=\"15\"/><sheetData>");
    xml.push_str("<row r=\"1\">");
    headers
        .iter()
        .enumerate()
        .for_each(|(column, header)| push_inline_cell(&mut xml, 1, column, header));
    xml.push_str("</row>");
    records
        .iter()
        .enumerate()
        .try_for_each(|(offset, record)| {
            let row = offset.saturating_add(2);
            xml.push_str(&format!("<row r=\"{row}\">"));
            append_record(&mut xml, row, &source_headers, record)?;
            xml.push_str("</row>");
            Ok::<(), anyhow::Error>(())
        })?;
    xml.push_str(&format!(
        "</sheetData><autoFilter ref=\"A1:{final_column}{final_row}\"/></worksheet>"
    ));
    Ok(xml)
}

fn append_record(
    xml: &mut String,
    row: usize,
    source_headers: &[String],
    record: &MatchRecord,
) -> Result<()> {
    source_headers
        .iter()
        .enumerate()
        .for_each(|(column, header)| {
            push_inline_cell(xml, row, column, source_field_value(record, header))
        });
    let values = result_values(record)?;
    values.iter().enumerate().for_each(|(offset, value)| {
        let column = source_headers.len().saturating_add(offset);
        if offset == 7 {
            push_number_cell(xml, row, column, value);
        } else if offset == 11 && !record.selected_profile_url.is_empty() {
            push_hyperlink_formula_cell(xml, row, column, &record.selected_profile_url);
        } else {
            push_inline_cell(xml, row, column, value);
        }
    });
    Ok(())
}

fn result_values(record: &MatchRecord) -> Result<Vec<String>> {
    Ok(vec![
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
        serde_json::to_string(&record.candidates)?,
        serde_json::to_string(&record.best_marks)?,
        record.notes.clone(),
        record.hint_count.to_string(),
        record.ai_logic.clone(),
    ])
}

fn source_field_value<'a>(record: &'a MatchRecord, header: &str) -> &'a str {
    record
        .prospect
        .source_fields
        .get(header)
        .map_or("", String::as_str)
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

pub(crate) fn next_relationship_id(xml: &str) -> Result<String> {
    let relationships = parse_relationships(xml)?;
    let maximum = relationships
        .keys()
        .try_fold(0_u32, |maximum, id| -> Result<u32> {
            let Some(raw_id) = id.strip_prefix("rId") else {
                return Ok(maximum);
            };
            let parsed = raw_id
                .parse::<u32>()
                .with_context(|| format!("invalid relationship id {id:?}"))?;
            Ok(maximum.max(parsed))
        })?;
    let next_id = maximum.checked_add(1).context("relationship id overflow")?;
    Ok(format!("rId{next_id}"))
}

pub(crate) fn insert_before(source: &str, marker: &str, insertion: &str) -> Result<String> {
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

pub(crate) fn temporary_output_path(output: &Path) -> PathBuf {
    let mut value = output.as_os_str().to_owned();
    value.push(format!(".{}.tmp", std::process::id()));
    PathBuf::from(value)
}
