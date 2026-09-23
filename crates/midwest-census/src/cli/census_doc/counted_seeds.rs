//! Distinct Athletic.net ids the external sources published, straight off the snapshots.

use std::collections::BTreeSet;
use std::path::Path;

use anyhow::{Context, Result};

use super::ns_key::ns_key;
use midwest_census::store::read::read_rows;

/// Data extracted from the athlete and meet JSONL snapshots.
pub(crate) struct Seeds {
    pub(super) athletes: usize,
    pub(super) multisource: usize,
    pub(super) distinct_athletic_net_athlete_ids: usize,
    pub(super) distinct_athletic_net_meet_ids: usize,
}

/// The rows of one snapshot the census document counts.
///
/// An absent file is an error here rather than an empty table: [`read_rows`] treats a snapshot that
/// has never been published as no rows, which is what an adapter wants before `consolidate` has run,
/// while this document is written after one. Counting a missing `athletes.jsonl` as zero athletes
/// would publish a number that contradicts the report sitting beside it.
fn snapshot_rows(path: &Path) -> Result<Vec<serde_json::Value>> {
    if !path.exists() {
        anyhow::bail!(
            "no snapshot at {}: consolidate the store before this document",
            path.display()
        );
    }
    read_rows(path).with_context(|| format!("reading {}", path.display()))
}

/// Process athlete rows: count total athletes, distinct AN athlete IDs, and multisource count.
fn count_athletes(rows: &[serde_json::Value]) -> (usize, usize, usize) {
    let mut an_athletes = BTreeSet::new();
    let mut athletes = 0usize;
    let mut multisource = 0usize;

    for row in rows {
        athletes = athletes.saturating_add(1);
        let mut namespaces = BTreeSet::new();
        if let Some(idents) = row.get("source_identities").and_then(|v| v.as_array()) {
            for identity in idents {
                let ns_val = identity
                    .get("namespace")
                    .unwrap_or(&serde_json::Value::Null);
                let key = ns_key(ns_val);
                namespaces.insert(key.clone());
                if key.starts_with("legacy_athletic_net") {
                    if let Some(id) = identity.get("id").and_then(|v| v.as_str()) {
                        an_athletes.insert(id.to_string());
                    }
                }
            }
        }
        if namespaces.len() > 1 {
            multisource = multisource.saturating_add(1);
        }
    }

    (athletes, multisource, an_athletes.len())
}

/// Process meet rows: count distinct AN meet IDs.
fn count_meet_ids(rows: &[serde_json::Value]) -> usize {
    let mut an_meets = BTreeSet::new();
    for row in rows {
        if let Some(idents) = row.get("source_identities").and_then(|v| v.as_array()) {
            for identity in idents {
                let ns_val = identity
                    .get("namespace")
                    .unwrap_or(&serde_json::Value::Null);
                let key = ns_key(ns_val);
                if key.starts_with("legacy_athletic_net") {
                    if let Some(id) = identity.get("id").and_then(|v| v.as_str()) {
                        an_meets.insert(id.to_string());
                    }
                }
            }
        }
    }
    an_meets.len()
}

pub(super) fn counted_seeds(store_out: &Path) -> Result<Seeds> {
    let athletes_path = store_out.join("athletes.jsonl");
    let athletes_rows = snapshot_rows(&athletes_path)?;
    let (athletes, multisource, distinct_athletic_net_athlete_ids) = count_athletes(&athletes_rows);

    let meets_path = store_out.join("meets.jsonl");
    let meets_rows = snapshot_rows(&meets_path)?;
    let distinct_athletic_net_meet_ids = count_meet_ids(&meets_rows);

    Ok(Seeds {
        athletes,
        multisource,
        distinct_athletic_net_athlete_ids,
        distinct_athletic_net_meet_ids,
    })
}

#[cfg(test)]
mod tests;
