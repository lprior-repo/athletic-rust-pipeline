use crate::model::{normalize_name, CanonicalSchool, SchoolId};
use crate::UsJurisdiction;
use std::collections::HashMap;

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
    exact: HashMap<(UsJurisdiction, String), SchoolId>,
    by_state: HashMap<UsJurisdiction, Vec<Entry>>,
}

impl SchoolIndex {
    pub fn from_schools(schools: &[CanonicalSchool]) -> Self {
        let mut exact = HashMap::new();
        let mut by_state: HashMap<UsJurisdiction, Vec<Entry>> = HashMap::new();
        for school in schools {
            let Some(state) = school.state else {
                continue;
            };
            exact
                .entry((state, school.normalized_name.clone()))
                .or_insert_with(|| school.id.clone());
            for alias in &school.aliases {
                exact
                    .entry((state, normalize_name(alias)))
                    .or_insert_with(|| school.id.clone());
            }
            by_state.entry(state).or_default().push(Entry {
                normalized: school.normalized_name.clone(),
                id: school.id.clone(),
            });
        }
        Self { exact, by_state }
    }

    pub fn school_count(&self) -> usize {
        self.by_state.values().map(Vec::len).sum()
    }

    pub fn resolve(&self, state: UsJurisdiction, raw: &str) -> Option<(SchoolId, SchoolMatch)> {
        let normalized = normalize_name(raw);
        if normalized.is_empty() {
            return None;
        }
        if let Some(id) = self.exact.get(&(state, normalized.clone())) {
            return Some((id.clone(), SchoolMatch::Exact));
        }
        for (pattern, replacement) in ABBREVIATIONS {
            if raw.to_ascii_lowercase().contains(pattern) {
                let expanded =
                    normalize_name(&raw.to_ascii_lowercase().replace(pattern, replacement));
                if let Some(id) = self.exact.get(&(state, expanded)) {
                    return Some((id.clone(), SchoolMatch::Abbreviation));
                }
            }
        }
        if let Some(without_squad) = without_squad_letter(&normalized) {
            if let Some(id) = self.exact.get(&(state, without_squad.to_string())) {
                return Some((id.clone(), SchoolMatch::Exact));
            }
            for (pattern, replacement) in ABBREVIATIONS {
                if raw.to_ascii_lowercase().contains(pattern) {
                    let expanded =
                        normalize_name(&raw.to_ascii_lowercase().replace(pattern, replacement));
                    let key =
                        without_squad_letter(&expanded).map_or(expanded.as_str(), |value| value);
                    if let Some(id) = self.exact.get(&(state, key.to_string())) {
                        return Some((id.clone(), SchoolMatch::Abbreviation));
                    }
                }
            }
            return self.partial(state, without_squad);
        }
        self.partial(state, &normalized)
    }

    fn partial(&self, state: UsJurisdiction, normalized: &str) -> Option<(SchoolId, SchoolMatch)> {
        let entries = self.by_state.get(&state)?;
        let (head, last) = normalized.rsplit_once(' ')?;
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
                    Some(existing) if *existing != entry.id => return None,
                    Some(_) => {}
                    None => matched = Some(entry.id.clone()),
                }
            }
        }
        matched.map(|id| (id, SchoolMatch::Partial))
    }
}

fn without_squad_letter(label: &str) -> Option<&str> {
    let (head, last) = label.rsplit_once(' ')?;
    (!head.is_empty() && last.len() == 1).then_some(head)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::CanonicalSchool;

    fn index() -> SchoolIndex {
        let names = [
            ("Milwaukee Bradley Tech", "milwaukee bradley tech"),
            ("Brookfield Central", "brookfield central"),
            ("Madison La Follette", "madison la follette"),
            ("Milwaukee King", "milwaukee king"),
            ("Wisconsin Lutheran", "wisconsin lutheran"),
            ("Eau Claire Memorial", "eau claire memorial"),
            ("Homestead", "homestead"),
            ("West De Pere", "west de pere"),
            ("Franklin", "franklin"),
        ];
        let schools: Vec<CanonicalSchool> = names
            .iter()
            .map(|(name, normalized)| {
                CanonicalSchool::new(UsJurisdiction::Wisconsin, *name, *normalized).0
            })
            .collect();
        SchoolIndex::from_schools(&schools)
    }

    #[test]
    fn exact_and_abbreviated_labels_resolve() {
        let index = index();
        assert_eq!(
            index
                .resolve(UsJurisdiction::Wisconsin, "West De Pere")
                .map(|(_, kind)| kind),
            Some(SchoolMatch::Exact)
        );
        assert_eq!(
            index
                .resolve(UsJurisdiction::Wisconsin, "Milw. Bradley Tech")
                .map(|(_, kind)| kind),
            Some(SchoolMatch::Abbreviation)
        );
    }

    #[test]
    fn truncated_and_prefixed_labels_resolve_to_the_unique_school() {
        let index = index();
        assert_eq!(
            index
                .resolve(UsJurisdiction::Wisconsin, "Brookfield Cent.")
                .map(|(_, kind)| kind),
            Some(SchoolMatch::Partial)
        );
        assert_eq!(
            index
                .resolve(UsJurisdiction::Wisconsin, "La Follette")
                .map(|(_, kind)| kind),
            Some(SchoolMatch::Partial)
        );
        assert_eq!(
            index
                .resolve(UsJurisdiction::Wisconsin, "Wisconsin Luth.")
                .map(|(_, kind)| kind),
            Some(SchoolMatch::Partial)
        );
        assert_eq!(
            index
                .resolve(UsJurisdiction::Wisconsin, "EC Memorial")
                .map(|(_, kind)| kind),
            Some(SchoolMatch::Abbreviation)
        );
    }

    #[test]
    fn relay_squad_letters_are_not_part_of_the_school_name() {
        let index = index();
        assert_eq!(
            index
                .resolve(UsJurisdiction::Wisconsin, "Homestead A")
                .map(|(_, kind)| kind),
            Some(SchoolMatch::Exact)
        );
        assert_eq!(
            index
                .resolve(UsJurisdiction::Wisconsin, "Homestead B")
                .map(|(_, kind)| kind),
            Some(SchoolMatch::Exact)
        );
        assert_eq!(
            index
                .resolve(UsJurisdiction::Wisconsin, "Milw. Bradley Tech A")
                .map(|(_, kind)| kind),
            Some(SchoolMatch::Abbreviation)
        );
    }

    #[test]
    fn ambiguous_labels_stay_unresolved() {
        let schools: Vec<CanonicalSchool> = ["Madison Memorial", "Milwaukee Memorial"]
            .iter()
            .map(|name| {
                CanonicalSchool::new(UsJurisdiction::Wisconsin, *name, normalize_name(name)).0
            })
            .collect();
        let index = SchoolIndex::from_schools(&schools);
        assert_eq!(index.resolve(UsJurisdiction::Wisconsin, "Memorial"), None);
    }

    #[test]
    fn matching_never_crosses_state_lines() {
        let index = index();
        assert_eq!(
            index.resolve(UsJurisdiction::Minnesota, "West De Pere"),
            None
        );
    }
}
