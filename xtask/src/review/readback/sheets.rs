use super::hash::hex_encode;

use anyhow::{Context, Result};
use calamine::{DataType, Reader};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::Path;

pub(super) fn verify_sheets(bundle_dir: &Path) -> Result<Vec<Value>> {
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
        let semantic_sha = compute_sheet_semantic_hash(&range)?;

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
    let mut count = 0usize;
    for row in range.rows() {
        for cell in row {
            if cell.as_string().is_some_and(|s| !s.is_empty()) {
                count = count.saturating_add(1);
            }
        }
    }
    count
}

fn compute_sheet_semantic_hash(range: &calamine::Range<calamine::Data>) -> Result<String> {
    let mut hasher = Sha256::new();

    let (start_row, start_col) = match range.start() {
        Some((row, col)) => (
            usize::try_from(row).context("sheet start row does not fit in usize")?,
            usize::try_from(col).context("sheet start column does not fit in usize")?,
        ),
        None => (0, 0),
    };

    for (row_idx, row) in range.rows().enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            let cell_row = start_row.saturating_add(row_idx);
            let cell_col = start_col.saturating_add(col_idx);

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
    Ok(hex_encode(&result))
}

pub(super) fn cell_reference(row: usize, col: usize) -> String {
    let col_letter = col_to_letter(col);
    format!("{}{}", col_letter, row.saturating_add(1))
}

fn col_to_letter(col: usize) -> String {
    let mut result = String::new();
    let mut n = col;
    loop {
        let digit = u8::try_from(n % 26).map_or(b'A', |offset| b'A'.saturating_add(offset));
        result.insert(0, char::from(digit));
        n /= 26;
        if n == 0 {
            break;
        }
        n = n.saturating_sub(1);
    }
    result
}
