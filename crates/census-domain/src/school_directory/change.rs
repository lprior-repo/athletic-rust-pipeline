use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::entry::SchoolDirectoryEntry;
use super::key::DirectoryKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum DirectoryField {
    Name,
    Address,
    City,
    State,
    Zip,
    Kind,
    Grades,
    Enrollment,
    Phone,
    Website,
}

impl DirectoryField {
    pub const ALL: [DirectoryField; 10] = [
        DirectoryField::Name,
        DirectoryField::Address,
        DirectoryField::City,
        DirectoryField::State,
        DirectoryField::Zip,
        DirectoryField::Kind,
        DirectoryField::Grades,
        DirectoryField::Enrollment,
        DirectoryField::Phone,
        DirectoryField::Website,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Address => "address",
            Self::City => "city",
            Self::State => "state",
            Self::Zip => "zip_code",
            Self::Kind => "kind",
            Self::Grades => "grades",
            Self::Enrollment => "enrollment",
            Self::Phone => "phone",
            Self::Website => "website",
        }
    }

    fn render(self, entry: &SchoolDirectoryEntry) -> Option<String> {
        match self {
            Self::Name => entry.name().map(|name| name.as_str().to_string()),
            Self::Address => entry.address().and_then(|address| {
                let mut rendered = address.line1().map(|line| line.as_str().to_string());
                if let Some(line2) = address.line2() {
                    let line2 = line2.as_str();
                    match &mut rendered {
                        Some(rendered) => {
                            rendered.push_str(", ");
                            rendered.push_str(line2);
                        }
                        None => rendered = Some(line2.to_string()),
                    }
                }
                rendered
            }),
            Self::City => entry
                .address()
                .and_then(|address| address.city())
                .map(|city| city.as_str().to_string()),
            Self::State => entry
                .address()
                .and_then(|address| address.state())
                .map(|state| state.code().to_string()),
            Self::Zip => entry
                .address()
                .and_then(|address| address.zip())
                .map(|zip| zip.to_string()),
            Self::Kind => entry.kind().map(|kind| kind.label()),
            Self::Grades => entry.grades().map(|grades| grades.label()),
            Self::Enrollment => entry.enrollment().map(|value| value.get().to_string()),
            Self::Phone => entry.phone().map(|phone| phone.as_str().to_string()),
            Self::Website => entry.website().map(|website| website.as_str().to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldDelta {
    pub field: DirectoryField,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Modification {
    pub key: DirectoryKey,
    pub label: String,
    pub deltas: Vec<FieldDelta>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ChangeSet {
    pub added: Vec<SchoolDirectoryEntry>,
    pub removed: Vec<SchoolDirectoryEntry>,
    pub modified: Vec<Modification>,
}

impl ChangeSet {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
    }

    pub fn total(&self) -> usize {
        self.added
            .len()
            .saturating_add(self.removed.len())
            .saturating_add(self.modified.len())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Baseline {
    entries: Vec<SchoolDirectoryEntry>,
}

impl Baseline {
    pub fn new(entries: Vec<SchoolDirectoryEntry>) -> Self {
        Self {
            entries: settle(entries),
        }
    }

    pub fn entries(&self) -> &[SchoolDirectoryEntry] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn diff(&self, next: &[SchoolDirectoryEntry]) -> ChangeSet {
        let next = settle(next.to_vec());
        let before = index(self.entries.iter());
        let after = index(next.iter());
        let added = after
            .iter()
            .filter(|(key, _)| !before.contains_key(*key))
            .map(|(_, entry)| (*entry).clone())
            .collect();
        let removed = before
            .iter()
            .filter(|(key, _)| !after.contains_key(*key))
            .map(|(_, entry)| (*entry).clone())
            .collect();
        let mut modified = Vec::new();
        for (key, old) in &before {
            let Some(new) = after.get(key) else {
                continue;
            };
            let deltas = deltas(old, new);
            if !deltas.is_empty() {
                modified.push(Modification {
                    key: (*key).clone(),
                    label: new.label(),
                    deltas,
                });
            }
        }
        ChangeSet {
            added,
            removed,
            modified,
        }
    }
}

fn settle(entries: Vec<SchoolDirectoryEntry>) -> Vec<SchoolDirectoryEntry> {
    let mut groups: BTreeMap<DirectoryKey, SchoolDirectoryEntry> = BTreeMap::new();
    for entry in entries {
        match groups.get_mut(entry.key()) {
            Some(settled) => settled.absorb(&entry),
            None => {
                groups.insert(entry.key().clone(), entry);
            }
        }
    }
    groups.into_values().collect()
}

fn index<'a>(
    entries: impl Iterator<Item = &'a SchoolDirectoryEntry>,
) -> BTreeMap<&'a DirectoryKey, &'a SchoolDirectoryEntry> {
    entries.map(|entry| (entry.key(), entry)).collect()
}

fn deltas(old: &SchoolDirectoryEntry, new: &SchoolDirectoryEntry) -> Vec<FieldDelta> {
    let mut deltas = Vec::new();
    for field in DirectoryField::ALL {
        let before = field.render(old);
        let after = field.render(new);
        if before != after {
            deltas.push(FieldDelta {
                field,
                before,
                after,
            });
        }
    }
    deltas
}
