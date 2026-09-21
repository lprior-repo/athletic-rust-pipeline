//! Canonical school index used by result-file adapters.
//!
//! Result files label schools the way the timer typed them: abbreviated (`Milw. Bradley Tech`),
//! truncated to a fixed column (`Brookfield Cent.`, `Wisconsin Luth.`), prefixed with a district
//! (`EC Memorial`), or suffixed with a relay squad letter (`Homestead A`). The index resolves those
//! labels onto canonical school ids so a result file mints athletes against the same ids the
//! association and roster adapters already use — reconciliation by construction rather than by a
//! later join.
//!
//! Match order, each step inside the school's own state:
//!
//! 1. exact normalized name;
//! 2. abbreviation-expanded exact name (`Milw.` → `Milwaukee`);
//! 3. the same two steps after dropping a trailing one-letter relay squad token;
//! 4. a **unique** token-aligned partial match: every token but the last must match exactly and the
//!    last token must be a prefix of the canonical token (`Brookfield Cent.` → `Brookfield
//!    Central`), or the canonical name must end with the label's tokens (`La Follette` → `Madison
//!    La Follette`).
//!
//! Anything else is reported as unresolved. A label that matches two schools is never guessed.

use census_domain::model::{normalize_name, CanonicalSchool, SchoolId};
use std::collections::HashMap;

/// How a school label was resolved onto a canonical school.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchoolMatch {
    /// The label normalizes to the canonical name.
    Exact,
    /// The label matched after expanding a timer abbreviation.
    Abbreviation,
    /// The label is a truncated or district-prefixed form of exactly one canonical name.
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

/// Timer abbreviations that never normalize onto the canonical spelling.
///
/// Only *word* abbreviations belong here. Punctuation variants (`St.` vs `St`) and trailing periods
/// already collapse in [`normalize_name`], so listing them would break a working exact match.
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

/// School label resolver over the consolidated school snapshot.
pub struct SchoolIndex {
    exact: HashMap<(String, String), SchoolId>,
    by_state: HashMap<String, Vec<Entry>>,
}

impl SchoolIndex {
    pub fn from_schools(schools: &[CanonicalSchool]) -> Self {
        let mut exact = HashMap::new();
        let mut by_state: HashMap<String, Vec<Entry>> = HashMap::new();
        for school in schools {
            let Some(state) = school.state.as_deref() else {
                continue;
            };
            let state = state.to_ascii_uppercase();
            exact
                .entry((state.clone(), school.normalized_name.clone()))
                .or_insert_with(|| school.id.clone());
            // Aliases resolve only when they do not collide with a real name.
            for alias in &school.aliases {
                exact
                    .entry((state.clone(), normalize_name(alias)))
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

    /// Resolve one raw school label observed in a result file.
    pub fn resolve(&self, state: &str, raw: &str) -> Option<(SchoolId, SchoolMatch)> {
        let state = state.to_ascii_uppercase();
        let normalized = normalize_name(raw);
        if normalized.is_empty() {
            return None;
        }
        if let Some(id) = self.exact.get(&(state.clone(), normalized.clone())) {
            return Some((id.clone(), SchoolMatch::Exact));
        }
        for (pattern, replacement) in ABBREVIATIONS {
            if raw.to_ascii_lowercase().contains(pattern) {
                let expanded =
                    normalize_name(&raw.to_ascii_lowercase().replace(pattern, replacement));
                if let Some(id) = self.exact.get(&(state.clone(), expanded)) {
                    return Some((id.clone(), SchoolMatch::Abbreviation));
                }
            }
        }
        // Relay squads are published as `<school> A`, `<school> B`, …; the letter is not part of
        // the school name.
        if let Some(without_squad) = without_squad_letter(&normalized) {
            if let Some(id) = self.exact.get(&(state.clone(), without_squad.to_string())) {
                return Some((id.clone(), SchoolMatch::Exact));
            }
            for (pattern, replacement) in ABBREVIATIONS {
                if raw.to_ascii_lowercase().contains(pattern) {
                    let expanded =
                        normalize_name(&raw.to_ascii_lowercase().replace(pattern, replacement));
                    let key = without_squad_letter(&expanded).unwrap_or(&expanded);
                    if let Some(id) = self.exact.get(&(state.clone(), key.to_string())) {
                        return Some((id.clone(), SchoolMatch::Abbreviation));
                    }
                }
            }
            return self.partial(&state, without_squad);
        }
        self.partial(&state, &normalized)
    }

    /// Token-aligned partial match; returns a candidate only when exactly one school matches.
    fn partial(&self, state: &str, normalized: &str) -> Option<(SchoolId, SchoolMatch)> {
        let entries = self.by_state.get(state)?;
        // A single-token label (`Memorial`, `Central`) is ambiguous by construction.
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

/// The label without its relay squad token: `Homestead A` → `Homestead`, and `None` when the last
/// token is not a one-letter squad (`Madison La Follette`) or there is no token to drop (`Franklin`).
fn without_squad_letter(label: &str) -> Option<&str> {
    let (head, last) = label.rsplit_once(' ')?;
    (!head.is_empty() && last.len() == 1).then_some(head)
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_domain::model::CanonicalSchool;

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
            .map(|(name, normalized)| CanonicalSchool::new("WI", *name, *normalized).0)
            .collect();
        SchoolIndex::from_schools(&schools)
    }

    #[test]
    fn exact_and_abbreviated_labels_resolve() {
        let index = index();
        assert_eq!(
            index.resolve("WI", "West De Pere").map(|(_, kind)| kind),
            Some(SchoolMatch::Exact)
        );
        assert_eq!(
            index
                .resolve("WI", "Milw. Bradley Tech")
                .map(|(_, kind)| kind),
            Some(SchoolMatch::Abbreviation)
        );
    }

    #[test]
    fn truncated_and_prefixed_labels_resolve_to_the_unique_school() {
        let index = index();
        // Hy-Tek truncates the school column and drops the district prefix.
        assert_eq!(
            index
                .resolve("WI", "Brookfield Cent.")
                .map(|(_, kind)| kind),
            Some(SchoolMatch::Partial)
        );
        assert_eq!(
            index.resolve("WI", "La Follette").map(|(_, kind)| kind),
            Some(SchoolMatch::Partial)
        );
        assert_eq!(
            index.resolve("WI", "Wisconsin Luth.").map(|(_, kind)| kind),
            Some(SchoolMatch::Partial)
        );
        assert_eq!(
            index.resolve("WI", "EC Memorial").map(|(_, kind)| kind),
            Some(SchoolMatch::Abbreviation)
        );
    }

    #[test]
    fn relay_squad_letters_are_not_part_of_the_school_name() {
        let index = index();
        assert_eq!(
            index.resolve("WI", "Homestead A").map(|(_, kind)| kind),
            Some(SchoolMatch::Exact)
        );
        assert_eq!(
            index.resolve("WI", "Homestead B").map(|(_, kind)| kind),
            Some(SchoolMatch::Exact)
        );
        // An abbreviated label carries the squad letter too: the letter drops after expansion.
        assert_eq!(
            index
                .resolve("WI", "Milw. Bradley Tech A")
                .map(|(_, kind)| kind),
            Some(SchoolMatch::Abbreviation)
        );
    }

    #[test]
    fn ambiguous_labels_stay_unresolved() {
        let schools: Vec<CanonicalSchool> = ["Madison Memorial", "Milwaukee Memorial"]
            .iter()
            .map(|name| CanonicalSchool::new("WI", *name, normalize_name(name)).0)
            .collect();
        let index = SchoolIndex::from_schools(&schools);
        assert_eq!(index.resolve("WI", "Memorial"), None);
    }

    #[test]
    fn matching_never_crosses_state_lines() {
        let index = index();
        assert_eq!(index.resolve("MN", "West De Pere"), None);
    }
}
