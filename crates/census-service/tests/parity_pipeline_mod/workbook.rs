use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result};
use calamine::{open_workbook_auto, Data, Reader};
use sha2::{Digest, Sha256};

use super::utils;

pub struct Workbook {
    pub bytes: Vec<u8>,
    pub parts: BTreeMap<String, u32>,
    pub shape: serde_json::Value,
}

impl Workbook {
    pub fn read(path: &Path, root: &Path) -> Result<Self> {
        let bytes = utils::read_bytes(path)?;
        let mut book = open_workbook_auto(path)
            .with_context(|| format!("opening {} with calamine", path.display()))?;
        let sheet_names = book.sheet_names().to_vec();
        ensure!(!sheet_names.is_empty(), "the workbook carries no sheets");

        let mut sheets: Vec<serde_json::Value> = Vec::with_capacity(sheet_names.len());
        let mut cells: Vec<String> = Vec::new();
        for name in &sheet_names {
            let range = book
                .worksheet_range(name)
                .with_context(|| format!("reading sheet {name}"))?;
            let mut sheet_cells: Vec<String> = Vec::new();
            for (row_index, row) in range.rows().enumerate() {
                let label = row.first().map(cell_text).unwrap_or_default();
                for (column, cell) in row.iter().enumerate() {
                    let text = cell_text(cell);
                    if text.is_empty() {
                        continue;
                    }
                    sheet_cells.push(format!(
                        "{name}!{row_index}:{column}={}",
                        volatile_cell(&label, column, text, root)
                    ));
                }
            }
            ensure!(
                !sheet_cells.is_empty(),
                "the workbook's {name} sheet carries no cell"
            );
            cells.extend(sheet_cells.iter().cloned());
            sheets.push(serde_json::json!({
                "name": name,
                "cells": sheet_cells.len(),
                "digest": digest(&sheet_cells)?,
            }));
        }
        ensure!(
            cells.len() > 100,
            "the workbook carries only {} non-empty cells, so the digest would be vacuous",
            cells.len()
        );

        let parts = utils::part_crcs(&bytes, path)?;
        let shape = serde_json::json!({
            "sheets": sheets,
            "parts": parts.keys().collect::<Vec<_>>(),
            "cells": cells.len(),
            "content_digest": digest(&cells)?,
        });
        Ok(Self {
            parts,
            shape,
            bytes,
        })
    }
}

fn cell_text(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(value) => value.clone(),
        Data::Float(value) => format!("{value}"),
        Data::Int(value) => format!("{value}"),
        Data::Bool(value) => format!("{value}"),
        other => other.to_string(),
    }
}

fn volatile_cell(label: &str, column: usize, text: String, root: &Path) -> String {
    if column != 1 {
        return text;
    }
    match label {
        "Core report generated" | "Workbook generated on" => "<date>".to_string(),
        "Store" | "Core note" | "Note" => text.replace(&root.display().to_string(), "<store>"),
        _ => text,
    }
}

fn digest(value: &[String]) -> Result<String> {
    let json = serde_json::to_string(value).context("serializing a value for its digest")?;
    let hash = Sha256::digest(json.as_bytes());
    let mut out = String::with_capacity(hash.len().saturating_mul(2));
    for byte in hash {
        out.push_str(&format!("{byte:02x}"));
    }
    Ok(out)
}

use anyhow::ensure;
