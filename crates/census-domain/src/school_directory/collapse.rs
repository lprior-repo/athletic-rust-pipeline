use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use super::entry::SchoolDirectoryEntry;
use super::key::{DirectoryKey, WeakKey};

const WEAK_RANK: u8 = 3;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollapseNote {
    pub weak: WeakKey,
    pub candidates: Vec<DirectoryKey>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CollapseOutcome {
    pub entries: Vec<SchoolDirectoryEntry>,
    pub notes: Vec<CollapseNote>,
}

pub fn collapse_entries(rows: Vec<SchoolDirectoryEntry>) -> CollapseOutcome {
    let mut entries = collapse_groups(rows);
    let mut notes: BTreeMap<WeakKey, BTreeSet<DirectoryKey>> = BTreeMap::new();
    loop {
        let (next, merged, pass_notes) = absorb_weak(entries);
        entries = next;
        for (weak, candidates) in pass_notes {
            notes.entry(weak).or_default().extend(candidates);
        }
        if !merged {
            break;
        }
    }
    entries.sort_by(|left, right| left.key().cmp(right.key()));
    let notes = notes
        .into_iter()
        .map(|(weak, candidates)| CollapseNote {
            weak,
            candidates: candidates.into_iter().collect(),
        })
        .collect();
    CollapseOutcome { entries, notes }
}

fn collapse_groups(rows: Vec<SchoolDirectoryEntry>) -> Vec<SchoolDirectoryEntry> {
    let mut groups: BTreeMap<DirectoryKey, Vec<SchoolDirectoryEntry>> = BTreeMap::new();
    for row in rows {
        groups.entry(row.key().clone()).or_default().push(row);
    }
    let mut entries: Vec<SchoolDirectoryEntry> = Vec::with_capacity(groups.len());
    for (_, group) in groups {
        let mut collapsed: Option<SchoolDirectoryEntry> = None;
        for row in group {
            match &mut collapsed {
                Some(entry) => entry.absorb(&row),
                None => collapsed = Some(row),
            }
        }
        if let Some(entry) = collapsed {
            entries.push(entry);
        }
    }
    entries
}

fn absorb_weak(
    entries: Vec<SchoolDirectoryEntry>,
) -> (
    Vec<SchoolDirectoryEntry>,
    bool,
    BTreeMap<WeakKey, BTreeSet<DirectoryKey>>,
) {
    let mut strong: Vec<SchoolDirectoryEntry> = Vec::new();
    let mut weak: Vec<(WeakKey, SchoolDirectoryEntry)> = Vec::new();
    for entry in entries {
        match entry.key() {
            DirectoryKey::Weak(key) => weak.push((key.clone(), entry)),
            _ => strong.push(entry),
        }
    }
    if weak.is_empty() {
        return (strong, false, BTreeMap::new());
    }
    let claims = claims_by_weak_key(&strong);
    let mut notes: BTreeMap<WeakKey, BTreeSet<DirectoryKey>> = BTreeMap::new();
    let mut merged = false;
    let mut kept: Vec<SchoolDirectoryEntry> = Vec::new();
    for (key, entry) in weak {
        match claims.get(&key) {
            None => kept.push(entry),
            Some(candidates) if candidates.len() == 1 => {
                let target = candidates.iter().next();
                match target.and_then(|target| find_mut(&mut strong, target)) {
                    Some(target) => {
                        target.absorb(&entry);
                        merged = true;
                    }
                    None => kept.push(entry),
                }
            }
            Some(candidates) => {
                notes
                    .entry(key)
                    .or_default()
                    .extend(candidates.iter().cloned());
                kept.push(entry);
            }
        }
    }
    strong.extend(kept);
    (strong, merged, notes)
}

fn claims_by_weak_key(
    strong: &[SchoolDirectoryEntry],
) -> BTreeMap<WeakKey, BTreeSet<DirectoryKey>> {
    let mut claims: BTreeMap<WeakKey, BTreeSet<DirectoryKey>> = BTreeMap::new();
    for entry in strong {
        if let Some(weak) = weak_key_of(entry) {
            claims.entry(weak).or_default().insert(entry.key().clone());
        }
    }
    claims
}

fn weak_key_of(entry: &SchoolDirectoryEntry) -> Option<WeakKey> {
    let name = entry.name()?;
    let city = entry.address().and_then(|address| address.city());
    let state = entry.address().and_then(|address| address.state());
    WeakKey::of(name, city, state).ok()
}

fn find_mut<'a>(
    entries: &'a mut [SchoolDirectoryEntry],
    key: &DirectoryKey,
) -> Option<&'a mut SchoolDirectoryEntry> {
    entries
        .iter_mut()
        .find(|entry| entry.key() == key && entry.key().rank() < WEAK_RANK)
}
