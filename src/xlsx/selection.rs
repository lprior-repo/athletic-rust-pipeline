use super::{prospect_from_record, walk_records};
use crate::model::{Prospect, SourceRecord, WorkbookStats};
use anyhow::{bail, Context, Result};
use std::path::Path;

#[derive(Debug)]
pub struct ScanResult {
    pub stats: WorkbookStats,
    pub prospects: Vec<Prospect>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScanMode {
    Sports(Vec<String>),
    Exhaustive,
    FirstWorksheet,
}

pub fn scan(
    path: &Path,
    mode: ScanMode,
    expected_graduation_year: Option<i32>,
) -> Result<ScanResult> {
    let prepared_mode = prepare_mode(&mode);
    let mut prospects = Vec::new();
    let mut selected_prospects = 0_u64;
    let mut stats = walk_records(path, &mode, |record| {
        if selects_prepared_record(&record, &prepared_mode) {
            prospects.push(prospect_from_record(record, expected_graduation_year));
            selected_prospects = selected_prospects.saturating_add(1);
        }
        Ok(())
    })?;
    validate_first_worksheet_headers(&mode, &stats)?;
    stats.selected_prospects = selected_prospects;
    Ok(ScanResult { stats, prospects })
}

fn prepare_mode(mode: &ScanMode) -> ScanMode {
    match mode {
        ScanMode::Sports(sports) => {
            ScanMode::Sports(sports.iter().map(|sport| normalize(sport)).collect())
        }
        ScanMode::Exhaustive => ScanMode::Exhaustive,
        ScanMode::FirstWorksheet => ScanMode::FirstWorksheet,
    }
}

fn validate_first_worksheet_headers(mode: &ScanMode, stats: &WorkbookStats) -> Result<()> {
    if !matches!(mode, ScanMode::FirstWorksheet) {
        return Ok(());
    }
    let headers = stats.sheets.first().map(|sheet| &sheet.headers);
    let has_required = headers.is_some_and(|values| {
        ["Person First", "Person Last"]
            .iter()
            .all(|header| values.iter().any(|value| value == header))
    });
    if !has_required {
        bail!("first worksheet lacks required Person First/Person Last headers");
    }
    Ok(())
}

pub(crate) fn select_sheets(
    sheets: Vec<super::SheetMeta>,
    mode: &ScanMode,
) -> Result<Vec<super::SheetMeta>> {
    if matches!(mode, ScanMode::FirstWorksheet) {
        let first = sheets
            .into_iter()
            .next()
            .context("workbook has no worksheets")?;
        if !is_source_sheet(&first.name) {
            bail!(
                "first worksheet is generated output, not source data: {}",
                first.name
            );
        }
        return Ok(vec![first]);
    }
    Ok(sheets
        .into_iter()
        .filter(|sheet| is_source_sheet(&sheet.name))
        .collect())
}

fn selects_prepared_record(record: &SourceRecord, mode: &ScanMode) -> bool {
    match mode {
        ScanMode::Exhaustive | ScanMode::FirstWorksheet => true,
        ScanMode::Sports(sports) => {
            let first_name = field(record, "Person First");
            let last_name = field(record, "Person Last");
            if first_name.trim().is_empty() && last_name.trim().is_empty() {
                return false;
            }
            sports.contains(&normalize(field(record, "Sports Sport")))
        }
    }
}

pub(crate) fn is_source_sheet(name: &str) -> bool {
    !["Athletic Matches", "Corrections", "Summary"]
        .iter()
        .any(|generated| name.eq_ignore_ascii_case(generated))
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
