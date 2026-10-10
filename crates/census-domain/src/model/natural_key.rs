use super::collision::{evidence_list, source_list};
use super::school::{city_key, same_city};
use super::{
    normalize_name, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, RetainedConflict, SourceIdentity,
};
use crate::jurisdiction::UsJurisdiction;

pub trait NaturalKey {
    fn same_natural_key(&self, other: &Self) -> bool;

    fn natural_key(&self) -> String;

    fn sources(&self) -> String;

    fn retained_conflicts(&self) -> &[RetainedConflict];
}

fn same_name(left: &str, right: &str) -> bool {
    left == right || normalize_name(left) == normalize_name(right)
}

fn same_owner(left: &Option<SourceIdentity>, right: &Option<SourceIdentity>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => left.namespace == right.namespace && left.id == right.id,
        (None, None) => true,
        _ => false,
    }
}

fn same_compressed(left: &str, right: &str) -> bool {
    left.chars()
        .filter(char::is_ascii_alphanumeric)
        .eq(right.chars().filter(char::is_ascii_alphanumeric))
}

fn compressed(name: &str) -> String {
    name.chars().filter(char::is_ascii_alphanumeric).collect()
}

impl NaturalKey for CanonicalSchool {
    fn same_natural_key(&self, other: &Self) -> bool {
        if self.state != other.state
            || !same_compressed(&self.normalized_name, &other.normalized_name)
        {
            return false;
        }
        match (self.city.as_deref(), other.city.as_deref()) {
            (Some(left), Some(right)) => same_city(left, right),
            _ => true,
        }
    }

    fn natural_key(&self) -> String {
        let state = self.state.map_or("??", UsJurisdiction::code);
        match self.city.as_deref() {
            Some(city) => format!(
                "school in {state} named {:?} in {:?}",
                compressed(&self.normalized_name),
                city_key(city)
            ),
            None => format!(
                "school in {state} named {:?}",
                compressed(&self.normalized_name)
            ),
        }
    }

    fn sources(&self) -> String {
        source_list(&self.source_identities.iter().collect::<Vec<_>>())
    }

    fn retained_conflicts(&self) -> &[RetainedConflict] {
        &self.retained_conflicts
    }
}

impl NaturalKey for CanonicalTeam {
    fn same_natural_key(&self, other: &Self) -> bool {
        self.school == other.school
            && self.sport == other.sport
            && self.gender == other.gender
            && self.school_year == other.school_year
    }

    fn natural_key(&self) -> String {
        format!(
            "team {:?} {:?} at {} in {}",
            self.sport,
            self.gender,
            self.school,
            self.school_year.get()
        )
    }

    fn sources(&self) -> String {
        source_list(&self.source_identities.iter().collect::<Vec<_>>())
    }

    fn retained_conflicts(&self) -> &[RetainedConflict] {
        &self.retained_conflicts
    }
}

impl NaturalKey for CanonicalCoach {
    fn same_natural_key(&self, other: &Self) -> bool {
        self.school == other.school
            && self.sport == other.sport
            && self.gender == other.gender
            && self.role == other.role
            && same_name(&self.name, &other.name)
    }

    fn natural_key(&self) -> String {
        format!(
            "coach {:?} at {} ({:?} {:?} {:?})",
            normalize_name(&self.name),
            self.school,
            self.sport,
            self.gender,
            self.role
        )
    }

    fn sources(&self) -> String {
        source_list(&self.source_identities.iter().collect::<Vec<_>>())
    }

    fn retained_conflicts(&self) -> &[RetainedConflict] {
        &self.retained_conflicts
    }
}

impl NaturalKey for CanonicalAthlete {
    fn same_natural_key(&self, other: &Self) -> bool {
        self.school == other.school
            && self.grad_year == other.grad_year
            && self.gender == other.gender
            && same_name(&self.canonical_name, &other.canonical_name)
            && same_owner(&self.source, &other.source)
    }

    fn natural_key(&self) -> String {
        format!(
            "athlete {:?} at {} (class {}, {:?})",
            normalize_name(&self.canonical_name),
            self.school,
            self.grad_year.get(),
            self.gender
        )
    }

    fn sources(&self) -> String {
        source_list(&self.identities().collect::<Vec<_>>())
    }

    fn retained_conflicts(&self) -> &[RetainedConflict] {
        &self.retained_conflicts
    }
}

impl NaturalKey for CanonicalMeet {
    fn same_natural_key(&self, other: &Self) -> bool {
        self.state == other.state && self.date == other.date && same_name(&self.name, &other.name)
    }

    fn natural_key(&self) -> String {
        let state = self
            .state
            .map_or(super::MEET_STATE_UNRESOLVED, UsJurisdiction::code);
        format!(
            "meet in {state} named {:?} on {}",
            normalize_name(&self.name),
            self.date
        )
    }

    fn sources(&self) -> String {
        source_list(&self.source_identities.iter().collect::<Vec<_>>())
    }

    fn retained_conflicts(&self) -> &[RetainedConflict] {
        &self.retained_conflicts
    }
}

impl NaturalKey for CanonicalEvent {
    fn same_natural_key(&self, other: &Self) -> bool {
        self.meet == other.meet
            && self.kind == other.kind
            && self.gender == other.gender
            && self.division.as_deref().map_or("", |value| value)
                == other.division.as_deref().map_or("", |value| value)
            && self.round.as_deref().map_or("", |value| value)
                == other.round.as_deref().map_or("", |value| value)
    }

    fn natural_key(&self) -> String {
        format!(
            "event {:?} of {} ({:?} division {:?} round {:?})",
            self.kind,
            self.meet,
            self.gender,
            self.division.as_deref().map_or("", |value| value),
            self.round.as_deref().map_or("", |value| value)
        )
    }

    fn sources(&self) -> String {
        evidence_list(&self.evidence)
    }

    fn retained_conflicts(&self) -> &[RetainedConflict] {
        &self.retained_conflicts
    }
}

impl NaturalKey for CanonicalPerformance {
    fn same_natural_key(&self, other: &Self) -> bool {
        self.athlete == other.athlete
            && self.meet == other.meet
            && self.date == other.date
            && self.source_key == other.source_key
    }

    fn natural_key(&self) -> String {
        format!(
            "performance of {} at {} on {} under source key {:?}",
            self.athlete, self.meet, self.date, self.source_key
        )
    }

    fn sources(&self) -> String {
        evidence_list(&self.evidence)
    }

    fn retained_conflicts(&self) -> &[RetainedConflict] {
        &self.retained_conflicts
    }
}
