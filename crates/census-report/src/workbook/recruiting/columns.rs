use census_domain::model::{CanonicalAthlete, ObservedGrade, SchoolYear, SourceNamespace};
use std::collections::BTreeSet;

use super::super::cells::Cell;

fn newest_observation(athlete: &CanonicalAthlete) -> Option<&ObservedGrade> {
    athlete
        .observed_grades
        .iter()
        .max_by_key(|o| (o.school_year.get(), o.grade.get()))
}

fn observed_season(athlete: &CanonicalAthlete) -> Option<SchoolYear> {
    newest_observation(athlete)
        .map(|observation| observation.school_year)
        .or_else(|| {
            athlete
                .evidence
                .iter()
                .filter_map(|evidence| SchoolYear::from_date(&evidence.observed_on))
                .max_by_key(|year| year.get())
        })
}

pub(super) fn observed_school_year(athlete: &CanonicalAthlete) -> Cell {
    observed_season(athlete).map_or(Cell::Empty, |year| Cell::text(year.short()))
}

pub(super) fn source_count(athlete: &CanonicalAthlete) -> usize {
    athlete
        .identities()
        .map(|identity| &identity.namespace)
        .collect::<BTreeSet<&SourceNamespace>>()
        .len()
}

pub(super) fn coverage_state(has_performance: bool, has_pr: bool) -> &'static str {
    match (has_performance, has_pr) {
        (true, true) => "pr",
        (true, false) => "performance",
        (false, _) => "identity-only",
    }
}

pub(super) fn published(value: Option<String>) -> Cell {
    Cell::text(value.map_or(Default::default(), core::convert::identity))
}

pub(super) fn flag(recorded: bool) -> Cell {
    if recorded {
        Cell::text("yes")
    } else {
        Cell::Empty
    }
}
