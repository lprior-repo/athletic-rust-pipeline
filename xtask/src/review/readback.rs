#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use calamine::{DataType, Reader};
use indexmap::IndexMap;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub(super) fn run(bundle: &Path, out: Option<&Path>) -> Result<()> {
    let bundle_dir = bundle;
    let manifest_path = bundle_dir.join("manifest.json");
    let manifest_content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading manifest {}", manifest_path.display()))?;
    let manifest: Value =
        serde_json::from_str(&manifest_content).with_context(|| "parsing manifest.json")?;

    let artifacts = verify_artifacts(bundle_dir, &manifest)?;
    let sheets = verify_sheets(bundle_dir)?;
    let sidecars = verify_sidecars(bundle_dir)?;

    let mismatches: Vec<String> = artifacts
        .iter()
        .filter(|a| {
            a["match"].as_bool() != Some(true)
                || a["missing"].as_bool().is_some_and(core::convert::identity)
        })
        .map(|a| {
            a["name"]
                .as_str()
                .map_or_else(|| "unknown".to_string(), str::to_string)
        })
        .collect();

    let output = json!({
        "bundle_dir": bundle_dir.display().to_string(),
        "manifest": {
            "schema_revision": manifest["schema_revision"],
            "generation_digest": manifest["generation_digest"],
            "lineage": manifest["lineage"],
            "selection": manifest["selection"]
        },
        "artifacts": artifacts,
        "sheets": sheets,
        "sidecars": sidecars,
        "mismatches": mismatches
    });

    let rendered =
        serde_json::to_string_pretty(&output).with_context(|| "serializing readback JSON")? + "\n";

    write_or_print(out, &rendered)?;

    if !mismatches.is_empty() {
        anyhow::bail!("{} artifact mismatch(es)", mismatches.len());
    }

    Ok(())
}

fn write_or_print(out: Option<&Path>, content: &str) -> Result<()> {
    match out {
        Some(path) => {
            std::fs::write(path, content).with_context(|| format!("writing {}", path.display()))
        }
        None => {
            print!("{}", content);
            Ok(())
        }
    }
}

fn verify_artifacts(bundle_dir: &Path, manifest: &Value) -> Result<Vec<Value>> {
    let artifacts = manifest["artifacts"]
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("manifest.artifacts is not an object"))?;

    let mut names: Vec<&str> = artifacts.keys().map(|k| k.as_str()).collect();
    names.sort();

    let mut results = Vec::new();
    for name in names {
        let info = &artifacts[name];
        let declared_bytes = info["bytes"].as_u64().map_or(0, core::convert::identity);
        let declared_sha = info["sha256"].as_str().map_or("", core::convert::identity);

        let file_path = bundle_dir.join(name);
        if !file_path.exists() {
            results.push(json!({
                "name": name,
                "declared_bytes": declared_bytes,
                "actual_bytes": 0,
                "declared_sha256": declared_sha,
                "actual_sha256": "",
                "match": false,
                "missing": true
            }));
            continue;
        }

        let (actual_bytes, actual_sha) =
            compute_file_hash(&file_path).with_context(|| format!("hashing {}", name))?;

        let match_result = actual_bytes == declared_bytes && actual_sha == declared_sha;

        results.push(json!({
            "name": name,
            "declared_bytes": declared_bytes,
            "actual_bytes": actual_bytes,
            "declared_sha256": declared_sha,
            "actual_sha256": actual_sha,
            "match": match_result,
            "missing": false
        }));
    }

    Ok(results)
}

fn compute_file_hash(path: &Path) -> Result<(u64, String)> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut reader = BufReader::new(&file);

    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    let mut total = 0u64;

    loop {
        let n = reader
            .read(&mut buf)
            .with_context(|| format!("reading {}", path.display()))?;
        if n == 0 {
            break;
        }
        total += n as u64;
        hasher.update(&buf[..n]);
    }

    let result = hasher.finalize();
    let hex = hex_encode(&result);

    Ok((total, hex))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut hex = String::new();
    for &b in bytes {
        hex.push_str(&format!("{:02x}", b));
    }
    hex
}

fn verify_sheets(bundle_dir: &Path) -> Result<Vec<Value>> {
    let workbook_path = bundle_dir.join("workbook.xlsx");
    if !workbook_path.exists() {
        anyhow::bail!("workbook not found at {}", workbook_path.display());
    }

    let mut book = calamine::open_workbook_auto(&workbook_path)
        .with_context(|| format!("opening workbook {}", workbook_path.display()))?;

    let sheet_names = book.sheet_names().to_vec();
    let mut results = Vec::new();

    for name in sheet_names {
        let range = book
            .worksheet_range(&name)
            .with_context(|| format!("reading sheet '{}'", name))?;

        let rows = range.height();
        let cells = count_nonempty_cells(&range);
        let semantic_sha = compute_sheet_semantic_hash(&range);

        results.push(json!({
            "name": name,
            "rows": rows,
            "cells": cells,
            "semantic_sha256": semantic_sha
        }));
    }

    Ok(results)
}

fn count_nonempty_cells(range: &calamine::Range<calamine::Data>) -> usize {
    let mut count = 0;
    for row in range.rows() {
        for cell in row {
            if cell.as_string().is_some_and(|s| !s.is_empty()) {
                count += 1;
            }
        }
    }
    count
}

fn compute_sheet_semantic_hash(range: &calamine::Range<calamine::Data>) -> String {
    let mut hasher = Sha256::new();

    let (start_row, start_col) = range
        .start()
        .map_or((0, 0), |(row, col)| (row as usize, col as usize));

    for (row_idx, row) in range.rows().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            let cell_row = start_row + row_idx;
            let cell_col = start_col + col_idx;

            if let Some(value) = cell.as_string() {
                if !value.is_empty() {
                    let ref_str = cell_reference(cell_row, cell_col);
                    let line = format!("{}={}\n", ref_str, value);
                    hasher.update(line.as_bytes());
                }
            }
        }
    }

    let result = hasher.finalize();
    hex_encode(&result)
}

fn cell_reference(row: usize, col: usize) -> String {
    let col_letter = col_to_letter(col);
    format!("{}{}", col_letter, row + 1)
}

fn col_to_letter(col: usize) -> String {
    let mut result = String::new();
    let mut n = col;
    loop {
        let digit = (n % 26) as u8 + b'A';
        result.insert(0, digit as char);
        n /= 26;
        if n == 0 {
            break;
        }
        n -= 1;
    }
    result
}

fn verify_sidecars(bundle_dir: &Path) -> Result<Value> {
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

fn count_csv_records(content: &str) -> usize {
    let records = count_csv_lines(content);
    let header = usize::from(content.lines().next().is_some());
    records.saturating_sub(header)
}

fn count_csv_lines(content: &str) -> usize {
    let mut records = 0;
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
            records += 1;
            ended_with_break = true;
        } else if c == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            records += 1;
            ended_with_break = true;
        } else {
            ended_with_break = false;
        }
    }

    if saw_char && !ended_with_break {
        records += 1;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_reference_computation() {
        assert_eq!(cell_reference(0, 0), "A1");
        assert_eq!(cell_reference(0, 1), "B1");
        assert_eq!(cell_reference(1, 0), "A2");
        assert_eq!(cell_reference(0, 25), "Z1");
        assert_eq!(cell_reference(0, 26), "AA1");
        assert_eq!(cell_reference(0, 27), "AB1");
        assert_eq!(cell_reference(0, 51), "AZ1");
        assert_eq!(cell_reference(0, 52), "BA1");
        assert_eq!(cell_reference(0, 701), "ZZ1");
        assert_eq!(cell_reference(0, 702), "AAA1");
    }

    #[test]
    fn csv_quoted_comma_not_split() {
        let content = "header\n\"a,b\",c\n";
        let records = count_csv_records(content);
        assert_eq!(records, 1);
    }

    #[test]
    fn csv_quoted_embedded_newline() {
        let content = "header\n\"a\nb\",c\n";
        let records = count_csv_records(content);
        assert_eq!(records, 1);
    }

    #[test]
    fn csv_crlf_line_endings() {
        let content = "header\r\nrow1\r\nrow2\r\n";
        let records = count_csv_records(content);
        assert_eq!(records, 2);
    }

    #[test]
    fn csv_header_only() {
        let content = "header\n";
        let records = count_csv_records(content);
        assert_eq!(records, 0);
    }

    #[test]
    fn csv_unterminated_final_record() {
        let content = "header\nrow1";
        let records = count_csv_records(content);
        assert_eq!(records, 1);
    }

    #[test]
    fn csv_empty_file() {
        let content = "";
        let records = count_csv_records(content);
        assert_eq!(records, 0);
    }

    #[test]
    fn hex_encode_roundtrip() {
        let data = [0x00, 0xff, 0xab, 0xcd];
        let hex = hex_encode(&data);
        assert_eq!(hex, "00ffabcd");
    }
}
