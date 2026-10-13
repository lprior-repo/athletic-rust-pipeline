use super::TestResult;
use crate::export::ExportDataset;
use crate::workbook::Options;
use calamine::{Data, Range, Reader};
use census_domain::model::GradYear;
use rust_xlsxwriter::Workbook;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub(in crate::workbook::verify) fn copy_bundle(path: &Path, target: &Path) -> TestResult<PathBuf> {
    let generation = path.parent().ok_or("generation")?;
    let name = generation.file_name().ok_or("generation name")?;
    let copy = target.join("generations").join(name);
    std::fs::create_dir_all(&copy)?;
    for entry in std::fs::read_dir(generation)? {
        let entry = entry?;
        std::fs::copy(entry.path(), copy.join(entry.file_name()))?;
    }
    Ok(copy.join("workbook.xlsx"))
}

pub(in crate::workbook::verify) fn alter_sheet(
    path: &Path,
    sheet: &str,
    alter: impl FnOnce(&mut Range<Data>) -> TestResult,
) -> TestResult {
    let mut source: calamine::Xlsx<_> = calamine::open_workbook(path)?;
    let names = source.sheet_names().to_vec();
    let mut workbook = Workbook::new();
    let mut alter = Some(alter);
    for name in names {
        let mut range = source.worksheet_range(&name)?;
        if name == sheet {
            alter.take().ok_or("duplicate target worksheet")?(&mut range)?;
        }
        write_sheet(&mut workbook, &name, &range)?;
    }
    check!(alter.is_none(), "missing worksheet {sheet}");
    workbook.save(path)?;
    Ok(())
}

fn write_sheet(workbook: &mut Workbook, name: &str, range: &Range<Data>) -> TestResult {
    let worksheet = workbook.add_worksheet();
    worksheet.set_name(name)?;
    let (first_row, first_column) = range.start().map_or((0, 0), |value| value);
    for (row, column, value) in range.used_cells() {
        let row = first_row
            .checked_add(u32::try_from(row)?)
            .ok_or("row overflow")?;
        let column = first_column
            .checked_add(u32::try_from(column)?)
            .ok_or("column overflow")?;
        let column = u16::try_from(column)?;
        match value {
            Data::String(value) => {
                worksheet.write_string(row, column, value)?;
            }
            Data::Float(value) => {
                worksheet.write_number(row, column, *value)?;
            }
            Data::Int(value) => {
                worksheet.write_number(row, column, f64::from(i32::try_from(*value)?))?;
            }
            Data::Bool(value) => {
                worksheet.write_boolean(row, column, *value)?;
            }
            other => return Err(format!("unsupported fixture cell {other:?}").into()),
        }
    }
    if let Some((last_row, _)) = range.end() {
        worksheet.set_row_height(last_row, 15)?;
    }
    Ok(())
}

pub(in crate::workbook::verify) fn remove_row(range: &mut Range<Data>, row: u32) -> TestResult {
    let (_, last_column) = range.end().ok_or("empty fixture range")?;
    for column in 0..=last_column {
        range.set_value((row, column), Data::Empty);
    }
    Ok(())
}

pub(in crate::workbook::verify) fn replace_cell(
    range: &mut Range<Data>,
    cell: (u32, u32),
    text: &str,
) {
    range.set_value(cell, Data::String(text.to_string()));
}

pub(in crate::workbook::verify) fn recapture(
    path: &Path,
    dataset: &ExportDataset,
    options: &Options,
) -> TestResult {
    let manifest_path = path.parent().ok_or("generation")?.join("manifest.json");
    let mut manifest: Value = serde_json::from_slice(&std::fs::read(&manifest_path)?)?;
    let revision = manifest
        .get("schema_revision")
        .and_then(Value::as_u64)
        .ok_or("manifest schema revision")?;
    let bytes = std::fs::read(path)?;
    let artifact = manifest
        .pointer_mut("/artifacts/workbook.xlsx")
        .ok_or("workbook artifact")?;
    *artifact = json!({"bytes": bytes.len(), "sha256": format!("{:x}", Sha256::digest(&bytes))});
    let selection = selection_json(options)?;
    let digest_input = format!(
        "[{revision},{},{},{}]",
        serde_json::to_string(&dataset.lineage)?,
        selection,
        serde_json::to_string(manifest.get("artifacts").ok_or("artifacts")?)?
    );
    *manifest.get_mut("generation_digest").ok_or("digest")? =
        json!(format!("{:x}", Sha256::digest(digest_input.as_bytes())));
    std::fs::write(manifest_path, serde_json::to_vec(&manifest)?)?;
    Ok(())
}

pub(in crate::workbook::verify) fn rejected_by_all(
    path: &Path,
    dataset: &ExportDataset,
    options: &Options,
    location: &str,
) -> TestResult {
    use crate::workbook::{publication, verify::verify_frozen};
    for result in [
        verify_frozen(path, dataset, options).map(|_| ()),
        publication::verify_published(path).map(|_| ()),
        publication::open_publication(path, &dataset.lineage, options.scope, GradYear::CO2027)
            .map(|_| ()),
    ] {
        let error = match result {
            Err(error) => error.to_string(),
            Ok(()) => return Err(format!("{location}: forged workbook accepted").into()),
        };
        check!(error.contains(location), "{location}: {error}");
    }
    Ok(())
}

fn selection_json(options: &Options) -> TestResult<String> {
    let mut jurisdictions: Vec<&str> = options
        .jurisdictions
        .iter()
        .map(|jurisdiction| jurisdiction.code())
        .collect();
    jurisdictions.sort_unstable();
    jurisdictions.dedup();
    let suffix = if jurisdictions.is_empty() {
        String::new()
    } else {
        format!(
            ",\"jurisdictions\":{}",
            serde_json::to_string(&jurisdictions)?
        )
    };
    Ok(format!(
        "{{\"scope\":{},\"grad_year\":{},\"limit\":{},\"school_year\":{}{suffix}}}",
        serde_json::to_string(options.scope.as_str())?,
        serde_json::to_string(&options.grad_year)?,
        serde_json::to_string(&options.limit)?,
        serde_json::to_string(&Some(options.school_year))?
    ))
}

#[test]
fn independent_selection_encoding_preserves_empty_and_canonical_state_scope() -> TestResult {
    use census_domain::{model::SchoolYear, UsJurisdiction};
    let mut options = Options {
        scope: crate::report::Scope::AllSources,
        grad_year: Some(2027),
        limit: None,
        school_year: SchoolYear::new(2026).ok_or("school year")?,
        out: None,
        jurisdictions: Vec::new(),
    };
    check!(eq; selection_json(&options)?, r#"{"scope":"all_sources","grad_year":2027,"limit":null,"school_year":2026}"#);
    options.jurisdictions = vec![
        UsJurisdiction::Wisconsin,
        UsJurisdiction::Michigan,
        UsJurisdiction::Wisconsin,
    ];
    check!(eq; selection_json(&options)?, r#"{"scope":"all_sources","grad_year":2027,"limit":null,"school_year":2026,"jurisdictions":["MI","WI"]}"#);
    Ok(())
}
