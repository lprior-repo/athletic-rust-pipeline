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
