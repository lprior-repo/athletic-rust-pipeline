use anyhow::{Context, Result};
use serde_json::Value;
use std::collections::BTreeMap;

use super::measure::{direction, file_entries, strings, union_keys, FileEntry};
use super::{
    CONTEXT_METRICS, FUNCTION_SITES, OVERSIZED_FILES, SOFT_STRUCTURE_METRICS, STRUCTURE_METRICS,
};
use crate::json::number;

pub(super) fn clippy(
    known: &Value,
    clippy: &BTreeMap<String, u64>,
    failures: &mut Vec<String>,
) -> Result<()> {
    let known_clippy = known
        .get("clippy")
        .and_then(Value::as_object)
        .context("the baseline has no `clippy` object")?;
    for key in union_keys(clippy, known_clippy) {
        let value = clippy.get(&key).copied().map_or(0, core::convert::identity);
        let was = number(known_clippy.get(&key));
        if value > was {
            failures.push(format!("clippy {key}: {was} -> {value}"));
        }
        if value != was {
            println!(
                "  clippy {key}: {was} -> {value} [{}]",
                direction(value, was)
            );
        }
    }
    Ok(())
}

pub(super) fn scan(scan: &Value, known: &Value, failures: &mut Vec<String>) -> Result<()> {
    let known_scan = known
        .get("scan")
        .and_then(Value::as_object)
        .context("the baseline has no `scan` object")?;
    let scan_crates = scan
        .get("crates")
        .and_then(Value::as_object)
        .context("the scan report has no `crates` object")?;
    for (name, counts) in scan_crates {
        let (Some(counts), before_crate) = (counts.as_object(), known_scan.get(name)) else {
            continue;
        };
        for (metric, value) in counts {
            let Some(value) = value.as_u64() else {
                continue;
            };
            let was = before_crate
                .and_then(Value::as_object)
                .and_then(|counts| counts.get(metric))
                .and_then(Value::as_u64)
                .map_or(0, core::convert::identity);
            let debt = !CONTEXT_METRICS.contains(&metric.as_str());
            if debt && value > was {
                failures.push(format!("scan {name}.{metric}: {was} -> {value}"));
            }
            if value != was {
                let note = if debt { "" } else { " (context)" };
                println!(
                    "  scan {name}.{metric}: {was} -> {value} [{}]{note}",
                    direction(value, was)
                );
            }
        }
    }
    Ok(())
}

pub(super) fn structure(scan: &Value, known: &Value, failures: &mut Vec<String>) -> Result<()> {
    let known_structure = known.get("structure").and_then(Value::as_object);
    let structure = scan
        .get("structure")
        .and_then(Value::as_object)
        .context("the scan report has no `structure` object")?;
    for metric in STRUCTURE_METRICS {
        let was = number(known_structure.and_then(|known| known.get(metric)));
        let value = number(structure.get(metric));
        if value != was {
            println!(
                "  structure {metric}: {was} -> {value} [{}]",
                direction(value, was)
            );
        }
        if value > was {
            failures.push(format!("structure {metric}: {was} -> {value}"));
            for site in strings(structure.get(FUNCTION_SITES)) {
                println!("    {site}");
            }
        }
    }
    for metric in SOFT_STRUCTURE_METRICS {
        let was = number(known_structure.and_then(|known| known.get(metric)));
        let value = number(structure.get(metric));
        if value != was {
            println!(
                "  structure {metric}: {was} -> {value} [{}] (target, not a budget)",
                direction(value, was)
            );
        }
    }

    oversized(
        &file_entries(known_structure.and_then(|known| known.get(OVERSIZED_FILES))),
        &file_entries(structure.get(OVERSIZED_FILES)),
        failures,
    );
    Ok(())
}

pub(super) fn raises(
    clippy: &BTreeMap<String, u64>,
    scan: &Value,
    old: &Value,
) -> Result<Vec<String>> {
    let mut raised: Vec<String> = Vec::new();
    raised_clippy(clippy, old, &mut raised);
    raised_scan(scan, old, &mut raised)?;
    raised_structure(scan, old, &mut raised)?;
    Ok(raised)
}

fn raised_clippy(clippy: &BTreeMap<String, u64>, old: &Value, raised: &mut Vec<String>) {
    let known = old.get("clippy").and_then(Value::as_object);
    for (key, value) in clippy {
        let before = number(known.and_then(|known| known.get(key)));
        if *value > before {
            raised.push(format!("clippy {key}: {before} -> {value}"));
        }
    }
}

fn raised_scan(scan: &Value, old: &Value, raised: &mut Vec<String>) -> Result<()> {
    let known = old.get("scan").and_then(Value::as_object);
    let scan_crates = scan
        .get("crates")
        .and_then(Value::as_object)
        .context("the scan report has no `crates` object")?;
    for (name, counts) in scan_crates {
        let Some(counts) = counts.as_object() else {
            continue;
        };
        let before_crate = known
            .and_then(|known| known.get(name))
            .and_then(Value::as_object);
        for (metric, value) in counts {
            let Some(value) = value.as_u64() else {
                continue;
            };
            let before = before_crate
                .and_then(|counts| counts.get(metric))
                .and_then(Value::as_u64)
                .map_or(0, core::convert::identity);
            if value > before {
                raised.push(format!("scan {name}.{metric}: {before} -> {value}"));
            }
        }
    }
    Ok(())
}

fn raised_structure(scan: &Value, old: &Value, raised: &mut Vec<String>) -> Result<()> {
    let old_structure = old.get("structure").and_then(Value::as_object);
    let structure = scan
        .get("structure")
        .and_then(Value::as_object)
        .context("the scan report has no `structure` object")?;
    let before_files = file_entries(old_structure.and_then(|old| old.get(OVERSIZED_FILES)));
    let known_files: BTreeMap<&str, &FileEntry> = before_files
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect();
    for entry in file_entries(structure.get(OVERSIZED_FILES)) {
        let Some(previous) = known_files.get(entry.path.as_str()) else {
            raised.push(format!(
                "structure {OVERSIZED_FILES}: new {}",
                entry.display
            ));
            continue;
        };
        let (Some(before), Some(after)) = (previous.count, entry.count) else {
            continue;
        };
        if after > before {
            raised.push(format!(
                "structure {OVERSIZED_FILES}: {}: {before} -> {after}",
                entry.path
            ));
        }
    }
    for metric in STRUCTURE_METRICS {
        let before = number(old_structure.and_then(|old| old.get(metric)));
        let after = number(structure.get(metric));
        if after > before {
            raised.push(format!("structure {metric}: {before} -> {after}"));
        }
    }
    Ok(())
}

fn oversized(known: &[FileEntry], current: &[FileEntry], failures: &mut Vec<String>) {
    let known_paths: BTreeMap<&str, &FileEntry> = known
        .iter()
        .map(|entry| (entry.path.as_str(), entry))
        .collect();
    for entry in current {
        let Some(before) = known_paths.get(entry.path.as_str()) else {
            failures.push(format!("new file over 300 lines: {}", entry.display));
            continue;
        };
        let (Some(before), Some(after)) = (before.count, entry.count) else {
            continue;
        };
        if after > before {
            failures.push(format!(
                "file over 300 lines grew: {}: {before} -> {after}",
                entry.path
            ));
        }
        if after != before {
            println!(
                "  file over 300 lines: {}: {before} -> {after} [{}]",
                entry.path,
                direction(after, before)
            );
        }
    }
    let current_paths: std::collections::BTreeSet<&str> =
        current.iter().map(|entry| entry.path.as_str()).collect();
    for entry in known {
        if !current_paths.contains(entry.path.as_str()) {
            println!("  file over 300 lines resolved: {} [DOWN]", entry.display);
        }
    }
    println!(
        "  files over 300 lines: {} -> {}",
        known.len(),
        current.len()
    );
}

#[cfg(test)]
#[path = "compare/tests.rs"]
mod tests;
