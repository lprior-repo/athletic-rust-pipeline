#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::Path;

mod artifacts;
mod hash;
mod sheets;
mod sidecars;

use artifacts::verify_artifacts;
use sheets::verify_sheets;
use sidecars::verify_sidecars;

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
            a.get("match").and_then(Value::as_bool) != Some(true)
                || a.get("missing")
                    .and_then(Value::as_bool)
                    .is_some_and(core::convert::identity)
        })
        .map(|a| {
            a.get("name")
                .and_then(Value::as_str)
                .map_or_else(|| "unknown".to_string(), str::to_string)
        })
        .collect();

    let output = json!({
        "bundle_dir": bundle_dir.display().to_string(),
        "manifest": {
            "schema_revision": manifest.get("schema_revision"),
            "generation_digest": manifest.get("generation_digest"),
            "lineage": manifest.get("lineage"),
            "selection": manifest.get("selection")
        },
        "artifacts": artifacts,
        "sheets": sheets,
        "sidecars": sidecars,
        "mismatches": mismatches
    });

    let mut rendered =
        serde_json::to_string_pretty(&output).with_context(|| "serializing readback JSON")?;
    rendered.push('\n');

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

#[cfg(test)]
mod tests {
    use super::hash::hex_encode;
    use super::sheets::cell_reference;
    use super::sidecars::count_csv_records;

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
