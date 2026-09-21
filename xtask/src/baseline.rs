//! The debt baseline: rewrite it from current measurements, and ratchet the measurements against it.
//!
//! The baseline is a ratchet. `update` refuses to raise any number unless `--allow-increase` says the
//! increase is deliberate; `ratchet` fails the gate when any metric grew, and prints every change as
//! `[DOWN]` or `[UP]` so burndown is visible on every run.
//!
//! The JSON shape is fixed: `note`, `clippy` (keyed `crate\tlint`), `scan` (keyed by crate) and
//! `structure`. `tools/quality-baseline.json` is read by tracked tooling, so the keys and their
//! nesting do not move.

use crate::json::{array_len, number, truthy};
use crate::paths;
use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// Written into every refreshed baseline, exactly as the deleted script wrote it.
const NOTE: &str = "Debt baseline for tools/gate.sh. Numbers may only shrink; refresh with tools/gate.sh --update-baseline after a burndown.";

/// The structure metrics the ratchet compares as numbers.
const STRUCTURE_METRICS: [&str; 2] = ["functions_over_60_lines", "functions_over_25_logical_lines"];

/// The structure key that holds one display entry per oversized file rather than a number.
const OVERSIZED_FILES: &str = "files_over_300_lines";

/// Rewrite `baseline` from the current clippy and scan measurements.
///
/// Refuses to raise a number without `allow_increase`: a burndown is the only legitimate reason for
/// the baseline to move down.
pub fn update(
    baseline: &Path,
    clippy_tsv: &Path,
    scan_json: &Path,
    allow_increase: bool,
) -> Result<()> {
    let clippy = clippy_counts(clippy_tsv)?;
    let scan = read_json(scan_json)?;
    let old = if baseline.exists() {
        read_json(baseline)?
    } else {
        Value::Object(Map::new())
    };

    if !allow_increase && truthy(&old) {
        let raised = raises(&clippy, &scan, &old)?;
        if !raised.is_empty() {
            println!("refusing to raise the baseline without --allow-increase:");
            for item in &raised {
                println!("  {item}");
            }
            bail!("the debt baseline was not written");
        }
    }

    let written = assemble(clippy, &scan)?;
    fs::write(baseline, written)
        .with_context(|| format!("writing {}", paths::relative(baseline)))?;
    println!("baseline updated: {}", paths::relative(baseline));
    Ok(())
}

/// Compare current measurements against `baseline`, failing when any metric grew.
pub fn ratchet(baseline: &Path, clippy_tsv: &Path, scan_json: &Path) -> Result<()> {
    let known = read_json(baseline)?;
    let clippy = clippy_counts(clippy_tsv)?;
    let scan = read_json(scan_json)?;
    let mut failures: Vec<String> = Vec::new();

    let known_clippy = known
        .get("clippy")
        .and_then(Value::as_object)
        .context("the baseline has no `clippy` object")?;
    for key in union_keys(&clippy, known_clippy) {
        let value = clippy.get(&key).copied().unwrap_or(0);
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
                .unwrap_or(0);
            if value > was {
                failures.push(format!("scan {name}.{metric}: {was} -> {value}"));
            }
            if value != was {
                println!(
                    "  scan {name}.{metric}: {was} -> {value} [{}]",
                    direction(value, was)
                );
            }
        }
    }

    let known_structure = known.get("structure").and_then(Value::as_object);
    let structure = scan
        .get("structure")
        .and_then(Value::as_object)
        .context("the scan report has no `structure` object")?;
    for metric in STRUCTURE_METRICS {
        let was = number(known_structure.and_then(|known| known.get(metric)));
        let value = number(structure.get(metric));
        if value > was {
            failures.push(format!("structure {metric}: {was} -> {value}"));
        }
        if value != was {
            println!(
                "  structure {metric}: {was} -> {value} [{}]",
                direction(value, was)
            );
        }
    }

    ratchet_oversized(
        &file_entries(known_structure.and_then(|known| known.get(OVERSIZED_FILES))),
        &file_entries(structure.get(OVERSIZED_FILES)),
        &mut failures,
    );

    if failures.is_empty() {
        println!("ratchet: no metric grew");
        return Ok(());
    }
    println!("ratchet failures (debt grew):");
    for item in &failures {
        println!("  {item}");
    }
    bail!(
        "debt grew in {} metric(s); see the failures above",
        failures.len()
    )
}

/// The oversized-file ledger: a new file is debt, a file that grew is debt, and a file that shrank
/// prints as burndown.
///
/// The deleted script compared the whole display entry (`crate:path/file.rs (412)`) as a set member,
/// which made every line-count change to an already-recorded file read as a brand new file: shrinking
/// `types.rs` from 408 to 404 lines failed the gate as "new file over 300 lines: ... (404)". Parsing
/// the entry keeps the intended check (the *path* is the identity) and adds the growth check the
/// string comparison only appeared to make.
fn ratchet_oversized(known: &[FileEntry], current: &[FileEntry], failures: &mut Vec<String>) {
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
    let current_paths: BTreeSet<&str> = current.iter().map(|entry| entry.path.as_str()).collect();
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

/// Every baseline number that the current measurements would raise.
fn raises(clippy: &BTreeMap<String, u64>, scan: &Value, old: &Value) -> Result<Vec<String>> {
    let empty = Map::new();
    let mut raised: Vec<String> = Vec::new();

    let old_clippy = old
        .get("clippy")
        .and_then(Value::as_object)
        .unwrap_or(&empty);
    for (key, value) in clippy {
        let before = number(old_clippy.get(key));
        if *value > before {
            raised.push(format!("clippy {key}: {before} -> {value}"));
        }
    }

    let old_scan = old.get("scan").and_then(Value::as_object).unwrap_or(&empty);
    let scan_crates = scan
        .get("crates")
        .and_then(Value::as_object)
        .context("the scan report has no `crates` object")?;
    for (name, counts) in scan_crates {
        let Some(counts) = counts.as_object() else {
            continue;
        };
        let before_crate = old_scan.get(name).and_then(Value::as_object);
        for (metric, value) in counts {
            let Some(value) = value.as_u64() else {
                continue;
            };
            let before = before_crate
                .and_then(|counts| counts.get(metric))
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if value > before {
                raised.push(format!("scan {name}.{metric}: {before} -> {value}"));
            }
        }
    }

    let old_structure = old.get("structure").and_then(Value::as_object);
    let structure = scan
        .get("structure")
        .and_then(Value::as_object)
        .context("the scan report has no `structure` object")?;
    let files_before = array_len(old_structure.and_then(|old| old.get(OVERSIZED_FILES)));
    let files_after = array_len(structure.get(OVERSIZED_FILES));
    if files_after > files_before {
        raised.push(format!(
            "structure {OVERSIZED_FILES}: {files_before} -> {files_after}"
        ));
    }
    for metric in STRUCTURE_METRICS {
        let before = number(old_structure.and_then(|old| old.get(metric)));
        let after = number(structure.get(metric));
        if after > before {
            raised.push(format!("structure {metric}: {before} -> {after}"));
        }
    }
    Ok(raised)
}

/// The refreshed baseline file: the fixed shape, with current measurements.
fn assemble(clippy: BTreeMap<String, u64>, scan: &Value) -> Result<String> {
    let crates = scan
        .get("crates")
        .cloned()
        .context("the scan report has no `crates` object")?;
    let structure = scan
        .get("structure")
        .cloned()
        .context("the scan report has no `structure` object")?;
    let mut baseline: Map<String, Value> = Map::new();
    baseline.insert("note".to_string(), Value::String(NOTE.to_string()));
    baseline.insert(
        "clippy".to_string(),
        Value::Object(
            clippy
                .into_iter()
                .map(|(key, value)| (key, Value::from(value)))
                .collect(),
        ),
    );
    baseline.insert("scan".to_string(), crates);
    baseline.insert("structure".to_string(), structure);
    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&Value::Object(baseline))?
    ))
}

/// The clippy tallies from the gate's `crate<TAB>lint<TAB>count` file.
fn clippy_counts(path: &Path) -> Result<BTreeMap<String, u64>> {
    let text =
        fs::read_to_string(path).with_context(|| format!("reading {}", paths::relative(path)))?;
    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let mut fields = line.split('\t');
        let (Some(name), Some(lint), Some(total)) = (fields.next(), fields.next(), fields.next())
        else {
            bail!(
                "{}: expected `crate<TAB>lint<TAB>count`, got {line:?}",
                paths::relative(path)
            );
        };
        let total = total
            .trim()
            .parse::<u64>()
            .with_context(|| format!("{}: parsing the count in {line:?}", paths::relative(path)))?;
        counts.insert(format!("{name}\t{lint}"), total);
    }
    Ok(counts)
}

/// The sorted union of the measured and the recorded clippy keys.
fn union_keys(clippy: &BTreeMap<String, u64>, known: &Map<String, Value>) -> BTreeSet<String> {
    let mut keys: BTreeSet<String> = clippy.keys().cloned().collect();
    keys.extend(known.keys().cloned());
    keys
}

/// The burndown arrow: `DOWN` when the metric shrank, `UP` when it grew.
fn direction(value: u64, was: u64) -> &'static str {
    if value < was {
        "DOWN"
    } else {
        "UP"
    }
}

/// One `files_over_300_lines` entry: the display string, the path it names, and its line count.
struct FileEntry {
    display: String,
    path: String,
    count: Option<u64>,
}

/// Parse every entry of a `files_over_300_lines` array.
fn file_entries(value: Option<&Value>) -> Vec<FileEntry> {
    value
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |entries| {
            entries
                .iter()
                .filter_map(Value::as_str)
                .map(parse_entry)
                .collect()
        })
}

/// `crate:path/to/file.rs (412)` as a path and a line count; an entry in any other shape keeps its
/// whole display string as the path and reports no count.
fn parse_entry(display: &str) -> FileEntry {
    let parsed = display.rsplit_once(" (").and_then(|(path, tail)| {
        let digits = tail.strip_suffix(')')?;
        Some((path, digits.parse::<u64>().ok()?))
    });
    let (path, count) = parsed.map_or((display, None), |(path, count)| (path, Some(count)));
    FileEntry {
        display: display.to_string(),
        path: path.to_string(),
        count,
    }
}

/// Read one JSON file, naming it when it cannot be read or parsed.
fn read_json(path: &Path) -> Result<Value> {
    let text =
        fs::read_to_string(path).with_context(|| format!("reading {}", paths::relative(path)))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", paths::relative(path)))
}
