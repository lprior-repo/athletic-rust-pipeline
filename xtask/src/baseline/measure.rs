//! Reading the two files a ratchet run compares: the gate's clippy tallies and the scan's report.
//!
//! Both are measurements produced by other lanes (`tools/gate.sh` writes the `crate<TAB>lint<TAB>count`
//! file, `scan` writes the JSON), so a file that cannot be read or parsed is an error naming the path
//! rather than an empty measurement — an empty measurement would read as a perfect burndown.
//!
//! The display helpers here turn a recorded entry back into what it names: an oversized-file entry is
//! stored as `crate:path/file.rs (412)`, and the ratchet needs the path and the count to tell a file
//! that grew from a file that was renamed.

use crate::paths;
use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// One `files_over_300_lines` entry: the display string, the path it names, and its line count.
pub(super) struct FileEntry {
    pub(super) display: String,
    pub(super) path: String,
    pub(super) count: Option<u64>,
}

/// The clippy tallies from the gate's `crate<TAB>lint<TAB>count` file.
pub(super) fn clippy_counts(path: &Path) -> Result<BTreeMap<String, u64>> {
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

/// Read one JSON file, naming it when it cannot be read or parsed.
pub(super) fn read_json(path: &Path) -> Result<Value> {
    let text =
        fs::read_to_string(path).with_context(|| format!("reading {}", paths::relative(path)))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", paths::relative(path)))
}

/// The sorted union of the measured and the recorded clippy keys.
pub(super) fn union_keys(
    clippy: &BTreeMap<String, u64>,
    known: &Map<String, Value>,
) -> BTreeSet<String> {
    let mut keys: BTreeSet<String> = clippy.keys().cloned().collect();
    keys.extend(known.keys().cloned());
    keys
}

/// Parse every entry of a `files_over_300_lines` array.
pub(super) fn file_entries(value: Option<&Value>) -> Vec<FileEntry> {
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

/// The burndown arrow: `DOWN` when the metric shrank, `UP` when it grew.
pub(super) fn direction(value: u64, was: u64) -> &'static str {
    if value < was {
        "DOWN"
    } else {
        "UP"
    }
}

/// The string entries of a structure array, for printing as evidence.
pub(super) fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
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
