use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{bail, Context, Result};
use calamine::{open_workbook, Data, Reader, Xlsx};
use serde_json::Value;

#[derive(Debug, PartialEq)]
pub struct Semantics {
    workbook: BTreeMap<String, Vec<Vec<Data>>>,
    sidecars: BTreeMap<String, Value>,
    csv: BTreeMap<String, Vec<Vec<String>>>,
}

pub fn read(workbook: &Path) -> Result<Semantics> {
    let directory = workbook
        .parent()
        .context("publication generation directory")?;
    let manifest: Value = serde_json::from_slice(&std::fs::read(directory.join("manifest.json"))?)?;
    let inventory = manifest
        .get("artifacts")
        .and_then(Value::as_object)
        .context("publication artifact inventory")?;
    let mut sidecars = BTreeMap::new();
    let mut csv = BTreeMap::new();
    for name in inventory.keys() {
        let path = directory.join(name);
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("xlsx") => {}
            Some("csv") => {
                let mut reader = csv::ReaderBuilder::new()
                    .has_headers(false)
                    .from_path(&path)?;
                let rows = reader
                    .records()
                    .map(|row| row.map(|row| row.iter().map(str::to_string).collect()))
                    .collect::<std::result::Result<Vec<Vec<String>>, csv::Error>>()?;
                csv.insert(name.clone(), rows);
            }
            Some("json" | "jsonl") => {
                sidecars.insert(name.clone(), read_json(&path)?);
            }
            extension => bail!("unhandled publication artifact {name}: {extension:?}"),
        }
    }
    Ok(Semantics {
        workbook: read_workbook(workbook)?,
        sidecars,
        csv,
    })
}

fn read_json(path: &Path) -> Result<Value> {
    let bytes = std::fs::read(path)?;
    if path.extension().and_then(|extension| extension.to_str()) == Some("jsonl") {
        let rows = serde_json::Deserializer::from_slice(&bytes)
            .into_iter::<Value>()
            .collect::<serde_json::Result<Vec<_>>>()?;
        return Ok(Value::Array(rows));
    }
    let mut value: Value = serde_json::from_slice(&bytes)?;
    match path.file_name().and_then(|name| name.to_str()) {
        Some("frozen-input.json") => {
            let lineage = value
                .get_mut("lineage")
                .and_then(Value::as_object_mut)
                .context("frozen input lineage")?;
            for field in [
                "store_identity",
                "input_generation",
                "source_digest",
                "snapshot_sequence",
                "generated_on",
            ] {
                lineage
                    .remove(field)
                    .with_context(|| format!("missing lineage field {field}"))?;
            }
        }
        Some("audit.json") => {
            value
                .as_object_mut()
                .context("publication audit object")?
                .remove("input_generation")
                .context("audit input generation")?;
        }
        Some("census-core.json" | "census-all-sources.json") => {
            value
                .as_object_mut()
                .context("census object")?
                .remove("generated_on")
                .context("census generation date")?;
        }
        _ => {}
    }
    Ok(value)
}

fn read_workbook(path: &Path) -> Result<BTreeMap<String, Vec<Vec<Data>>>> {
    let mut workbook: Xlsx<_> = open_workbook(path)?;
    let mut sheets = BTreeMap::new();
    for name in workbook.sheet_names().to_vec() {
        let range = workbook.worksheet_range(&name)?;
        let mut rows: Vec<Vec<Data>> = range.rows().map(<[Data]>::to_vec).collect();
        if name == "Run Metrics" {
            for row in &mut rows {
                if matches!(row.first(), Some(Data::String(label)) if matches!(label.as_str(),
                    "Store identity" | "Input generation" | "Source content digest"
                        | "Captured store sequence" | "Workbook generated on"))
                {
                    *row.get_mut(1).context("run metric value")? = Data::Empty;
                }
            }
        }
        sheets.insert(name, rows);
    }
    Ok(sheets)
}
