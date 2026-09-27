
use census_domain::model::{CanonicalAthlete, ObservedGrade, SourceNamespace};
use std::collections::{BTreeMap, BTreeSet};

use super::super::cells::Cell;

fn newest_observation(athlete: &CanonicalAthlete) -> Option<&ObservedGrade> {
    athlete
        .observed_grades
        .iter()
        .max_by_key(|o| (o.school_year.get(), o.grade.get()))
}

pub(super) fn observed_grade(athlete: &CanonicalAthlete) -> Cell {
    newest_observation(athlete)
        .map(|o| Cell::text(o.grade.to_string()))
        .unwrap_or(Cell::Empty)
}

pub(super) fn observed_school_year(athlete: &CanonicalAthlete) -> Cell {
    newest_observation(athlete)
        .map(|o| Cell::text(o.school_year.short()))
        .unwrap_or(Cell::Empty)
}

pub(super) fn source_count(athlete: &CanonicalAthlete) -> usize {
    athlete.identities()
        .map(|identity| &identity.namespace)
        .collect::<BTreeSet<&SourceNamespace>>()
        .len()
}

pub(super) fn conflicts(athlete: &CanonicalAthlete) -> bool {
    athlete
        .observed_grades
        .iter()
        .any(|observation| observation.grad_year() != athlete.grad_year)
        || conflicting_identities(athlete)
}

fn conflicting_identities(athlete: &CanonicalAthlete) -> bool {
    let mut seen: BTreeMap<&SourceNamespace, &str> = BTreeMap::new();
    for identity in athlete.identities() { match seen.get(&identity.namespace) {
        Some(existing) if *existing != identity.id.as_str() => return true,
        Some(_) => {}
        None => {
            seen.insert(&identity.namespace, identity.id.as_str());
        }
    } }
    false
}

pub(super) fn coverage_state(has_performance: bool, has_pr: bool) -> &'static str {
    match (has_performance, has_pr) {
        (true, true) => "pr",
        (true, false) => "performance",
        (false, _) => "identity-only",
    }
}

pub(super) fn published(value: Option<String>) -> Cell {
    Cell::text(value.unwrap_or_default())
}

pub(super) fn flag(recorded: bool) -> Cell {
    if recorded {
        Cell::text("yes")
    } else {
        Cell::Empty
    }
}
