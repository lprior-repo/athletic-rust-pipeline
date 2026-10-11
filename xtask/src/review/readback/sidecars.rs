use anyhow::{Context, Result};
use indexmap::IndexMap;
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn verify_sidecars(bundle_dir: &Path) -> Result<Value> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(bundle_dir)
        .with_context(|| format!("listing {}", bundle_dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "manifest.json" || name == "workbook.xlsx" {
            continue;
        }
        files.push(path);
    }

    files.sort();

    let mut sidecars = IndexMap::new();
    for path in files {
        let name = path.file_name().map_or_else(
            || "unknown".to_string(),
            |f| f.to_string_lossy().to_string(),
        );
        let info = process_sidecar(&path, &name)?;
        sidecars.insert(name, info);
    }

    Ok(serde_json::to_value(sidecars)?)
}

fn process_sidecar(path: &Path, name: &str) -> Result<Value> {
    let content = std::fs::read_to_string(path).with_context(|| format!("reading {}", name))?;

    if name.ends_with(".csv") {
        let records = count_csv_records(&content);
        return Ok(json!({
            "records": records,
            "bytes": content.len()
        }));
    }

    if name.ends_with(".jsonl") {
        let lines = content.lines().filter(|l| !l.trim().is_empty()).count();
        return Ok(json!({
            "records": lines,
            "bytes": content.len()
        }));
    }

    if name.ends_with(".json") {
        let parsed: Value =
            serde_json::from_str(&content).with_context(|| format!("parsing {}", name))?;
        let records = json_value_record_count(&parsed);
        let root_type = json_value_root_type(&parsed);
        return Ok(json!({
            "records": records,
            "bytes": content.len(),
            "root_type": root_type
        }));
    }

    Ok(json!({
        "bytes": content.len()
    }))
}

pub(super) fn count_csv_records(content: &str) -> usize {
    let records = count_csv_lines(content);
    let header = usize::from(content.lines().next().is_some());
    records.saturating_sub(header)
}

fn count_csv_lines(content: &str) -> usize {
    let mut records = 0usize;
    let mut in_quotes = false;
    let mut saw_char = false;
    let mut ended_with_break = false;
    let mut chars = content.chars().peekable();

    while let Some(c) = chars.next() {
        saw_char = true;
        if in_quotes {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                } else {
                    in_quotes = false;
                }
            }
            ended_with_break = false;
        } else if c == '"' {
            in_quotes = true;
            ended_with_break = false;
        } else if c == '\n' {
            records = records.saturating_add(1);
            ended_with_break = true;
        } else if c == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            records = records.saturating_add(1);
            ended_with_break = true;
        } else {
            ended_with_break = false;
        }
    }

    if saw_char && !ended_with_break {
        records = records.saturating_add(1);
    }

    records
}

fn json_value_record_count(value: &Value) -> usize {
    match value {
        Value::Array(arr) => arr.len(),
        Value::Object(_) => 1,
        _ => 1,
    }
}

fn json_value_root_type(value: &Value) -> &str {
    match value {
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Bool(_) => "boolean",
        Value::Null => "null",
    }
}
