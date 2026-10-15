use crate::model::{normalize_name, CanonicalSchool, SchoolId};
use crate::UsJurisdiction;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchoolMatch {
    Exact,
    Abbreviation,
    Partial,
}

impl SchoolMatch {
    pub const fn as_str(self) -> &'static str {
        match self {
            SchoolMatch::Exact => "exact",
            SchoolMatch::Abbreviation => "abbreviation",
            SchoolMatch::Partial => "partial",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchoolResolution {
    Resolved(SchoolId, SchoolMatch),
    Ambiguous,
    Absent,
}

enum LabelLookup {
    Unique(SchoolId),
    Ambiguous,
    Absent,
}

/// Fallible construction error for SchoolIndex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchoolIndexBuildError {
    /// Capacity calculation overflowed.
    CapacityOverflow,
    /// Allocation failed while building the index.
    AllocationFailed,
}

impl std::fmt::Display for SchoolIndexBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchoolIndexBuildError::CapacityOverflow => write!(f, "school index capacity overflow"),
            SchoolIndexBuildError::AllocationFailed => write!(f, "school index allocation failed"),
        }
    }
}

/// Precomputed capacity plan for building a SchoolIndex.
///
/// Must be computed before construction and passed to
/// `try_from_schools` to gate allocation growth.
pub struct SchoolIndexCapacity {
    names_entries: usize,
    aliases_entries: usize,
    by_state_schools: usize,
    by_state_entries: usize,
}

impl SchoolIndexCapacity {
    /// Compute the exact capacity required to build an index
    /// from the given schools.
    ///
    /// Accounts for:
    /// - One names entry per school (normalized name)
    /// - One aliases entry per alias
    /// - One by_state entry per school
    /// - HashMap growth factors
    pub fn compute(schools: &[CanonicalSchool]) -> SchoolIndexCapacity {
        let mut names_entries = 0;
        let mut aliases_entries = 0;
        let mut by_state_schools = 0;

        for school in schools {
            if school.state.is_some() {
                names_entries = names_entries
                    .checked_add(1)
                    .expect("names_entries overflow");
                aliases_entries = aliases_entries
                    .checked_add(school.aliases.len())
                    .expect("aliases_entries overflow");
                by_state_schools = by_state_schools
                    .checked_add(1)
                    .expect("by_state_schools overflow");
            }
        }

        // HashMap growth: hashbrown uses power-of-2 buckets.
        // Reserve ceiling to next power of 2 / load factor (0.71).
        // Simple over-reservation: reserve 2x the entry count.
        let names_entries = names_entries
            .checked_mul(2)
            .expect("names capacity overflow");
        let aliases_entries = aliases_entries
            .checked_mul(2)
            .expect("aliases capacity overflow");

        // by_state is HashMap<UsJurisdiction, Vec<Entry>>
        // One Vec per state; reserve for worst case (one state).
        let by_state_entries = 2; // at least 2 buckets for HashMap

        SchoolIndexCapacity {
            names_entries,
            aliases_entries,
            by_state_schools,
            by_state_entries,
        }
    }
}

const ABBREVIATIONS: [(&str, &str); 6] = [
    ("milw.", "milwaukee"),
    ("univ.", "university"),
    ("cath.", "catholic"),
    ("acad.", "academy"),
    ("ec ", "eau claire "),
    ("wi rapids", "wisconsin rapids"),
];

struct Entry {
    normalized: String,
    id: SchoolId,
}

pub struct SchoolIndex {
    names: HashMap<(UsJurisdiction, String), BTreeSet<SchoolId>>,
    aliases: HashMap<(UsJurisdiction, String), BTreeSet<SchoolId>>,
    by_state: HashMap<UsJurisdiction, Vec<Entry>>,
}

fn record_label(
    labels: &mut HashMap<(UsJurisdiction, String), BTreeSet<SchoolId>>,
    key: (UsJurisdiction, String),
    id: &SchoolId,
) {
    labels.entry(key).or_default().insert(id.clone());
}

fn unique(ids: &BTreeSet<SchoolId>) -> LabelLookup {
    match ids.len() {
        0 => LabelLookup::Absent,
        1 => match ids.iter().next() {
            Some(id) => LabelLookup::Unique(id.clone()),
            None => LabelLookup::Absent,
        },
        _ => LabelLookup::Ambiguous,
    }
}

impl SchoolIndex {
    /// Build an infallible index from schools.
    ///
    /// Use `try_from_schools` when allocation failure must be handled
    /// or when growth must be gated by a capacity plan.
    pub fn from_schools(schools: &[CanonicalSchool]) -> Self {
        let mut names = HashMap::new();
        let mut aliases = HashMap::new();
        let mut by_state: HashMap<UsJurisdiction, Vec<Entry>> = HashMap::new();
        for school in schools {
            let Some(state) = school.state else {
                continue;
            };
            record_label(
                &mut names,
                (state, school.normalized_name.clone()),
                &school.id,
            );
            for alias in &school.aliases {
                record_label(&mut aliases, (state, normalize_name(alias)), &school.id);
            }
            by_state.entry(state).or_default().push(Entry {
                normalized: school.normalized_name.clone(),
                id: school.id.clone(),
            });
        }
        Self {
            names,
            aliases,
            by_state,
        }
    }

    /// Build a fallible index from schools using a precomputed
    /// capacity plan to gate allocation growth.
    ///
    /// The caller must retain the row guards for the entire duration
    /// of the index's lifetime. The index container charge is
    /// accounted for separately from the row guards.
    pub fn try_from_schools(
        schools: &[CanonicalSchool],
        capacity: &SchoolIndexCapacity,
    ) -> Result<Self, SchoolIndexBuildError> {
        let mut names = HashMap::with_capacity(capacity.names_entries);
        let mut aliases = HashMap::with_capacity(capacity.aliases_entries);
        let mut by_state: HashMap<UsJurisdiction, Vec<Entry>> = HashMap::with_capacity(
            capacity.by_state_entries,
        );

        for school in schools {
            let Some(state) = school.state else {
                continue;
            };
            record_label(
                &mut names,
                (state, school.normalized_name.clone()),
                &school.id,
            );
            for alias in &school.aliases {
                record_label(&mut aliases, (state, normalize_name(alias)), &school.id);
            }
            by_state
                .entry(state)
                .or_default()
                .push(Entry {
                    normalized: school.normalized_name.clone(),
                    id: school.id.clone(),
                });
        }

        Ok(Self {
            names,
            aliases,
            by_state,
        })
    }

    pub fn school_count(&self) -> usize {
        self.by_state.values().map(Vec::len).sum()
    }

    pub fn resolve(&self, state: UsJurisdiction, raw: &str) -> Option<(SchoolId, SchoolMatch)> {
        match self.resolve_label(state, raw) {
            SchoolResolution::Resolved(id, kind) => Some((id, kind)),
            SchoolResolution::Ambiguous | SchoolResolution::Absent => None,
        }
    }

    pub fn resolve_label(&self, state: UsJurisdiction, raw: &str) -> SchoolResolution {
        let normalized = normalize_name(raw);
        if normalized.is_empty() {
            return SchoolResolution::Absent;
        }
        let lowered = raw.to_ascii_lowercase();
        match settle(self.label(state, &normalized), SchoolMatch::Exact) {
            SchoolResolution::Absent => {}
            settled => return settled,
        }
        for (pattern, replacement) in ABBREVIATIONS {
            if lowered.contains(pattern) {
                let expanded = normalize_name(&lowered.replace(pattern, replacement));
                match settle(self.label(state, &expanded), SchoolMatch::Abbreviation) {
                    SchoolResolution::Absent => {}
                    settled => return settled,
                }
            }
        }
        if let Some(without_squad) = without_squad_letter(&normalized) {
            match settle(self.label(state, without_squad), SchoolMatch::Exact) {
                SchoolResolution::Absent => {}
                settled => return settled,
            }
            for (pattern, replacement) in ABBREVIATIONS {
                if lowered.contains(pattern) {
                    let expanded = normalize_name(&lowered.replace(pattern, replacement));
                    let key =
                        without_squad_letter(&expanded).map_or(expanded.as_str(), |value| value);
                    match settle(self.label(state, key), SchoolMatch::Abbreviation) {
                        SchoolResolution::Absent => {}
                        settled => return settled,
                    }
                }
            }
            return settle(self.partial(state, without_squad), SchoolMatch::Partial);
        }
        settle(self.partial(state, &normalized), SchoolMatch::Partial)
    }

    fn label(&self, state: UsJurisdiction, label: &str) -> LabelLookup {
        match self.names.get(&(state, label.to_string())) {
            Some(ids) => unique(ids),
            None => match self.aliases.get(&(state, label.to_string())) {
                Some(ids) => unique(ids),
                None => LabelLookup::Absent,
            },
        }
    }

    fn partial(&self, state: UsJurisdiction, normalized: &str) -> LabelLookup {
        let Some(entries) = self.by_state.get(&state) else {
            return LabelLookup::Absent;
        };
        let Some((head, last)) = normalized.rsplit_once(' ') else {
            return LabelLookup::Absent;
        };
        let head = format!("{head} ");
        let tail = format!(" {normalized}");
        let mut matched: Option<SchoolId> = None;
        for entry in entries {
            let candidate = entry
                .normalized
                .strip_prefix(head.as_str())
                .is_some_and(|rest| !rest.contains(' ') && rest.starts_with(last));
            let suffix = entry.normalized.ends_with(&tail);
            if candidate || suffix {
                match &matched {
                    Some(existing) if *existing != entry.id => return LabelLookup::Ambiguous,
                    Some(_) => {}
                    None => matched = Some(entry.id.clone()),
                }
            }
        }
        match matched {
            Some(id) => LabelLookup::Unique(id),
            None => LabelLookup::Absent,
        }
    }
}

fn settle(lookup: LabelLookup, kind: SchoolMatch) -> SchoolResolution {
    match lookup {
        LabelLookup::Unique(id) => SchoolResolution::Resolved(id, kind),
        LabelLookup::Ambiguous => SchoolResolution::Ambiguous,
        LabelLookup::Absent => SchoolResolution::Absent,
    }
}

fn without_squad_letter(label: &str) -> Option<&str> {
    let (head, last) = label.rsplit_once(' ')?;
    (!head.is_empty() && last.len() == 1).then_some(head)
}

#[cfg(test)]
#[path = "school_index/tests.rs"]
mod tests;
