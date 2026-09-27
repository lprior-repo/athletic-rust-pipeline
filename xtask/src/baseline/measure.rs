
use crate::paths;
use anyhow::{bail, Context, Result};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub(super) struct FileEntry {
    pub(super) display: String,
    pub(super) path: String,
    pub(super) count: Option<u64>,
}

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

pub(super) fn read_json(path: &Path) -> Result<Value> {
    let text =
        fs::read_to_string(path).with_context(|| format!("reading {}", paths::relative(path)))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", paths::relative(path)))
}

pub(super) fn union_keys(
    clippy: &BTreeMap<String, u64>,
    known: &Map<String, Value>,
) -> BTreeSet<String> {
    let mut keys: BTreeSet<String> = clippy.keys().cloned().collect();
    keys.extend(known.keys().cloned());
    keys
}

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

pub(super) fn direction(value: u64, was: u64) -> &'static str {
    if value < was {
        "DOWN"
    } else {
        "UP"
    }
}

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
