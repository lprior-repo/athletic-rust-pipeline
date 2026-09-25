//! The §50 cell rules shared by the `Athletes` sheet: grade, identity, coverage and primitive
//! renderers, split out of the sheet module to keep both files inside the budget.
//!
//! Each rule is a pure function over stored fields; the sheet module says which column prints which.
//! `observed_grade` and `observed_school_year` return the newest `ObservedGrade` — evidence (§3),
//! never a re-derivation of the cohort — and `coverage_state` states which stored performances
//! the athlete supports.
//!
//! **Rule: current-year grade.** A grade observation is treated as the athlete's current standing
//! only when the school year matches the current season (`SchoolYear::DEFAULT`). For all other
//! observations the grade is historical evidence and is printed alongside its school year so a
//! recruiter can see both the evidence and when it was captured. An athlete whose newest
//! observation is grade 11 in 2025-26 is NOT relabelled as grade 12.

use census_domain::model::{CanonicalAthlete, ObservedGrade, SourceNamespace};
use std::collections::{BTreeMap, BTreeSet};

use super::super::cells::Cell;

/// The most recent grade observation for the athlete, newest school year first.
///
/// Returns `None` when the athlete carries no grade observations at all.
/// Observations are sorted by school year descending, then by grade descending.
fn newest_observation(athlete: &CanonicalAthlete) -> Option<&ObservedGrade> {
    athlete
        .observed_grades
        .iter()
        .max_by_key(|o| (o.school_year.get(), o.grade.get()))
}

/// The latest grade observation the athlete carries — the grade and the school year it was
/// observed in, so a recruiter can see both the evidence and when it was captured.
///
/// For a class-of-2027 athlete with observed grades 9 (2023-24), 10 (2024-25), 11 (2025-26), 12
/// (2026-27) this returns `(12, "2026-27")`; for an athlete whose last observation was grade 11 in
/// 2025-26 it returns `(11, "2025-26")` rather than fabricating a "12".
pub(super) fn observed_grade(athlete: &CanonicalAthlete) -> Cell {
    newest_observation(athlete)
        .map(|o| Cell::text(o.grade.to_string()))
        .unwrap_or(Cell::Empty)
}

/// The school year in which the newest grade observation was captured, e.g. `"2025-26"`.
///
/// Blank when no grade observation exists.
pub(super) fn observed_school_year(athlete: &CanonicalAthlete) -> Cell {
    newest_observation(athlete)
        .map(|o| Cell::text(o.school_year.short()))
        .unwrap_or(Cell::Empty)
}

/// How many distinct source namespaces the athlete is known through — the census's own
/// "multiple sources" rule.
pub(super) fn source_count(athlete: &CanonicalAthlete) -> usize {
    athlete
        .source_identities
        .iter()
        .map(|identity| &identity.namespace)
        .collect::<BTreeSet<&SourceNamespace>>()
        .len()
}

/// Whether the athlete carries a stored disagreement: a grade observation that implies another
/// graduating class, or one source namespace carrying two different ids.
pub(super) fn conflicts(athlete: &CanonicalAthlete) -> bool {
    athlete
        .observed_grades
        .iter()
        .any(|observation| observation.grad_year() != athlete.grad_year)
        || conflicting_identities(athlete)
}

/// One source namespace holding two different external ids for this athlete: the merge kept both
/// observations and cannot decide which identity is right.
fn conflicting_identities(athlete: &CanonicalAthlete) -> bool {
    let mut seen: BTreeMap<&SourceNamespace, &str> = BTreeMap::new();
    for identity in &athlete.source_identities {
        match seen.get(&identity.namespace) {
            Some(existing) if *existing != identity.id.as_str() => return true,
            Some(_) => {}
            None => {
                seen.insert(&identity.namespace, identity.id.as_str());
            }
        }
    }
    false
}

/// Which coverage answer the athlete's stored rows support: a `PRs` row, stored performances without
/// a comparable PR (relay legs and unparsed marks), or no performance at all.
pub(super) fn coverage_state(has_performance: bool, has_pr: bool) -> &'static str {
    match (has_performance, has_pr) {
        (true, true) => "pr",
        (true, false) => "performance",
        (false, _) => "identity-only",
    }
}

/// One optional published value, or a blank when the store holds none.
pub(super) fn published(value: Option<String>) -> Cell {
    Cell::text(value.unwrap_or_default())
}

/// `yes` for a recorded fact, blank otherwise.
pub(super) fn flag(recorded: bool) -> Cell {
    if recorded {
        Cell::text("yes")
    } else {
        Cell::Empty
    }
}
