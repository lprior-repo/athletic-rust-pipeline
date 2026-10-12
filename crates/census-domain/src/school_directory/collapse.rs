use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::entry::SchoolDirectoryEntry;
use super::key::{DirectoryKey, WeakKey};
use super::DirectoryError;

#[path = "collapse_limits.rs"]
mod limits;

#[cfg(test)]
pub(super) mod metrics {
    std::thread_local! {
        pub static COMPARISONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    }
    pub fn comparison() {
        COMPARISONS.with(|count| count.set(count.get().saturating_add(1)));
    }
}

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

pub fn collapse_entries(
    rows: Vec<SchoolDirectoryEntry>,
) -> Result<CollapseOutcome, DirectoryError> {
    limits::input(&rows)?;
    let mut entries = collapse_groups(rows)?;
    let mut notes: HashMap<WeakKey, Vec<DirectoryKey>> = HashMap::new();
    let mut work = 0;
    loop {
        limits::work(&mut work, entries.len())?;
        let (next, merged) = absorb_weak(entries, &mut notes, &mut work)?;
        entries = next;
        if !merged {
            break;
        }
    }
    entries.sort_unstable_by(|left, right| left.key().cmp(right.key()));
    let mut ordered_notes = Vec::new();
    limits::reserve(&mut ordered_notes, notes.len())?;
    for (weak, mut candidates) in notes {
        candidates.sort_unstable();
        candidates.dedup();
        ordered_notes.push(CollapseNote { weak, candidates });
    }
    ordered_notes.sort_unstable_by(|left, right| left.weak.cmp(&right.weak));
    Ok(CollapseOutcome {
        entries,
        notes: ordered_notes,
    })
}

fn collapse_groups(
    rows: Vec<SchoolDirectoryEntry>,
) -> Result<Vec<SchoolDirectoryEntry>, DirectoryError> {
    let mut positions: HashMap<DirectoryKey, usize> = HashMap::new();
    positions
        .try_reserve(rows.len())
        .map_err(|_| DirectoryError::Allocation {
            resource: "collapse groups",
        })?;
    let mut entries: Vec<SchoolDirectoryEntry> = Vec::new();
    limits::reserve(&mut entries, rows.len())?;
    for row in rows {
        match positions.get(row.key()).copied() {
            Some(position) => {
                let count = entries.len();
                let entry = entries.get_mut(position).ok_or(DirectoryError::Capacity {
                    resource: "collapse group position",
                    requested: position,
                    limit: count,
                })?;
                entry.absorb(&row);
            }
            None => {
                positions.insert(row.key().clone(), entries.len());
                entries.push(row);
            }
        }
    }
    entries.sort_unstable_by(|left, right| left.key().cmp(right.key()));
    Ok(entries)
}

fn absorb_weak(
    entries: Vec<SchoolDirectoryEntry>,
    notes: &mut HashMap<WeakKey, Vec<DirectoryKey>>,
    work: &mut usize,
) -> Result<(Vec<SchoolDirectoryEntry>, bool), DirectoryError> {
    let mut strong = Vec::new();
    let mut weak = Vec::new();
    limits::reserve(&mut strong, entries.len())?;
    limits::reserve(&mut weak, entries.len())?;
    for entry in entries {
        match entry.key() {
            DirectoryKey::Weak(key) => weak.push((key.clone(), entry)),
            _ => strong.push(entry),
        }
    }
    if weak.is_empty() {
        return Ok((strong, false));
    }
    let claims = claims_by_weak_key(&strong)?;
    let mut merged = false;
    for (key, entry) in weak {
        match claims.get(&key).map(Vec::as_slice) {
            Some([position]) => {
                #[cfg(test)]
                metrics::comparison();
                let count = strong.len();
                let target = strong.get_mut(*position).ok_or(DirectoryError::Capacity {
                    resource: "collapse target position",
                    requested: *position,
                    limit: count,
                })?;
                target.absorb(&entry);
                merged = true;
            }
            Some(candidates) => {
                record_note(notes, key, candidates, &strong, work)?;
                strong.push(entry);
            }
            None => strong.push(entry),
        }
    }
    Ok((strong, merged))
}

fn record_note(
    notes: &mut HashMap<WeakKey, Vec<DirectoryKey>>,
    key: WeakKey,
    positions: &[usize],
    strong: &[SchoolDirectoryEntry],
    work: &mut usize,
) -> Result<(), DirectoryError> {
    limits::work(work, positions.len())?;
    notes
        .try_reserve(1)
        .map_err(|_| DirectoryError::Allocation {
            resource: "collapse notes",
        })?;
    let candidates = notes.entry(key).or_default();
    limits::reserve(candidates, positions.len())?;
    for position in positions {
        let entry = strong.get(*position).ok_or(DirectoryError::Capacity {
            resource: "collapse note position",
            requested: *position,
            limit: strong.len(),
        })?;
        candidates.push(entry.key().clone());
    }
    Ok(())
}

fn claims_by_weak_key(
    strong: &[SchoolDirectoryEntry],
) -> Result<HashMap<WeakKey, Vec<usize>>, DirectoryError> {
    let mut claims: HashMap<WeakKey, Vec<usize>> = HashMap::new();
    claims
        .try_reserve(strong.len())
        .map_err(|_| DirectoryError::Allocation {
            resource: "collapse claims",
        })?;
    for (position, entry) in strong.iter().enumerate() {
        #[cfg(test)]
        metrics::comparison();
        if let Some(weak) = weak_key_of(entry) {
            let positions = claims.entry(weak).or_default();
            limits::reserve(positions, 1)?;
            positions.push(position);
        }
    }
    Ok(claims)
}

fn weak_key_of(entry: &SchoolDirectoryEntry) -> Option<WeakKey> {
    let name = entry.name()?;
    let city = entry.address().and_then(|address| address.city());
    let state = entry.address().and_then(|address| address.state());
    WeakKey::of(name, city, state).ok()
}
