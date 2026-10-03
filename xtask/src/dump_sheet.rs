#![forbid(unsafe_code)]

use anyhow::Context;
use calamine::{DataType, Reader};
use std::path::Path;
const MAX_CELL_LEN: usize = 240;

pub fn run(workbook_path: &Path, sheet_names: &[String]) -> anyhow::Result<()> {
    let mut book = calamine::open_workbook_auto(workbook_path)
        .with_context(|| format!("opening workbook {}", workbook_path.display()))?;

    for name in sheet_names {
        if sheet_names.len() > 1 {
            eprintln!("=== {} ===", name);
        }

        let range = book
            .worksheet_range(name)
            .with_context(|| format!("reading sheet '{name}'"))?;
        let mut row_iter = range.rows();
        let headers: Vec<String>;

        if let Some(first_row) = row_iter.next() {
            let first_strs: Vec<String> = first_row
                .iter()
                .map(|c| c.as_string().map_or(Default::default(), core::convert::identity))
                .collect();

            let is_header = !first_strs.iter().all(|s| s.is_empty());

            if is_header {
                let header_fields: Vec<String> = first_strs
                    .iter()
                    .map(|s| truncate(s, MAX_CELL_LEN))
                    .collect();
                println!("{}", header_fields.join(" "));
            }

            headers = first_strs;

            for row in row_iter {
                if !row_has_text(row) {
                    continue;
                }
                let fields: Vec<String> = headers
                    .iter()
                    .zip(row.iter())
                    .map(|(col, cell)| {
                        let value = cell.as_string().map_or(Default::default(), core::convert::identity);
                        let truncated = truncate(&value, MAX_CELL_LEN);
                        format!("{col}={truncated}")
                    })
                    .collect();
                println!("{}", fields.join(" "));
            }
        }
    }

    Ok(())
}

fn row_has_text(row: &[impl calamine::DataType]) -> bool {
    row.iter()
        .any(|cell| cell.as_string().is_some_and(|s| !s.is_empty()))
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        let truncated: String = s.chars().take(max).collect();
        format!("{truncated}…")
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncation_keeps_prefix() {
        let long = "a".repeat(500);
        let result = truncate(&long, MAX_CELL_LEN);
        assert_eq!(result.chars().count(), MAX_CELL_LEN + 1);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn short_value_unaffected() {
        let result = truncate("hello", MAX_CELL_LEN);
        assert_eq!(result, "hello");
    }
}
