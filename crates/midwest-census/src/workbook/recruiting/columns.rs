//! The §50 column rules that are not contact columns: the grade, tally, identity and coverage cells
//! the `Athletes` sheet prints, split out of the sheet module to keep both files inside the budget.
//!
//! Each rule is a pure function over the athlete's stored fields and the athlete's tally; the sheet
//! module says which column prints which. `current_grade` reads the newest `ObservedGrade` — evidence
//! (§3), never a re-derivation of the cohort — and `coverage_state` states which of the three
//! published coverage answers the athlete's stored performances support.

use census_domain::model::{CanonicalAthlete, SourceNamespace};
use std::collections::{BTreeMap, BTreeSet};

use super::super::cells::Cell;
use super::facts::AthleteTally;
use super::prs::PrRow;

/// How many of an athlete's PRs the headline column names.
const HEADLINE_PRS: usize = 5;

/// The grade of the most recently observed grade entry, newest school year first and then the higher
/// grade; blank when the cohort came from a source that published no grade.
pub(super) fn current_grade(athlete: &CanonicalAthlete) -> Cell {
    athlete
        .observed_grades
        .iter()
        .max_by_key(|observation| (observation.school_year.get(), observation.grade.get()))
        .map(|observation| Cell::text(observation.grade.to_string()))
        .unwrap_or(Cell::Empty)
}

/// The event kinds the athlete has a stored mark in, in the tally's own order.
pub(super) fn event_list(tally: Option<&AthleteTally>) -> String {
    tally
        .map(|tally| {
            tally
                .events
                .iter()
                .cloned()
                .collect::<Vec<String>>()
                .join("; ")
        })
        .unwrap_or_default()
}

/// The first [`HEADLINE_PRS`] PRs in the `PRs` sheet's own order for this athlete, so the headline
/// and the PR rows agree.
pub(super) fn headline(prs: &[&PrRow]) -> String {
    prs.iter()
        .take(HEADLINE_PRS)
        .map(|pr| format!("{} {}", pr.event, pr.mark))
        .collect::<Vec<String>>()
        .join("; ")
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
