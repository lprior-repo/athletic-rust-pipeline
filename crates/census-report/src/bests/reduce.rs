mod build;
mod combined;
mod grouping;
mod identity;
mod reported;
mod reports;
mod row;
mod support;

#[cfg(test)]
pub(super) use build::build_outcome;
pub(crate) use build::{build_admitted_outcome, build_from_derivation, build_selected_outcome};
pub use build::{build_from_dataset, build_with_exclusions};
pub(crate) use reported::{reported_best_groups, ReportedBestGroup};

use grouping::ReportGroupKey;
use reports::{ContextIntegrity, ReportGroup, RetainedContexts};

use super::selection::{Population, SharedSelection};
use super::{CombinedExclusionReason, Parents};
use crate::report::{EventAdmission, Scope};
use census_domain::model::performance_comparison::{should_replace, MarkOrdering, Measure, PrKey};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, EventSpecification,
    SourceIdentity,
};
use std::collections::{BTreeMap, HashMap};

struct PrSlot<'a> {
    sources: Vec<&'a SourceIdentity>,
    reports: BTreeMap<ReportGroupKey<'a>, ReportGroup<'a>>,
    marks: usize,
}

impl<'a> PrSlot<'a> {
    fn empty() -> Self {
        Self {
            sources: Vec::new(),
            reports: BTreeMap::new(),
            marks: 0,
        }
    }

    fn candidate(&self) -> Option<&Candidate<'a>> {
        self.reports
            .values()
            .filter_map(ReportGroup::eligible)
            .reduce(|incumbent, candidate| {
                if candidate.wins_over(incumbent) {
                    candidate
                } else {
                    incumbent
                }
            })
    }
}

#[derive(Clone, Copy)]
struct Candidate<'a> {
    event: &'a CanonicalEvent,
    athlete: &'a CanonicalAthlete,
    meet: Option<&'a CanonicalMeet>,
    performance: &'a CanonicalPerformance,
    measure: Measure,
    value: i64,
}

impl<'a> Candidate<'a> {
    fn resolve(
        performance: &'a CanonicalPerformance,
        parents: &Parents<'a>,
        admission: &EventAdmission<'_>,
    ) -> Option<Result<(PrKey, Self), CombinedExclusionReason>> {
        let athlete = parents.athlete(performance.athlete.as_str())?;
        let event = parents.canonical_event(performance.event.as_str())?;
        let Some(kind) = admission.included_kind(&performance.event) else {
            return combined::unresolved_specification(performance, event, admission).map(Err);
        };
        let (measure, value) = eligible_measure(performance, kind)?;
        let meet = parents.meet(performance.meet.as_str());
        if let Err(reason) = combined::admit(performance, event, kind, admission.semantics()) {
            return Some(Err(reason));
        }
        let disposition = admission.disposition(&event.id)?;
        let key = PrKey::from_admitted(
            performance,
            event,
            meet,
            measure,
            &athlete.id,
            disposition,
            admission.semantics(),
        )?;
        Some(Ok((
            key,
            Self {
                event,
                athlete,
                meet,
                performance,
                measure,
                value,
            },
        )))
    }

    fn record(
        self,
        entry: &mut PrSlot<'a>,
        integrity: ContextIntegrity,
        policy: census_domain::model::EventSemantics,
    ) {
        entry.marks = entry.marks.saturating_add(1);
        if let Some(owner) = owner_of(self.performance, self.athlete) {
            if !entry.sources.contains(&owner) {
                entry.sources.push(owner);
            }
        }
        let context = ReportGroupKey::of(self.performance, policy);
        match entry.reports.entry(context) {
            std::collections::btree_map::Entry::Vacant(vacant) => {
                vacant.insert(ReportGroup::new(self, integrity));
            }
            std::collections::btree_map::Entry::Occupied(mut occupied) => {
                occupied.get_mut().record(self, integrity, policy);
            }
        }
    }

    fn wins_over(&self, incumbent: &Self) -> bool {
        should_replace(
            self.value,
            incumbent.value,
            self.ordering(),
            incumbent.ordering(),
            |candidate, incumbent| self.measure.better(candidate, incumbent),
        )
    }

    fn ordering(&self) -> MarkOrdering<'_> {
        MarkOrdering::new(&self.performance.date, self.performance.id.as_str())
    }
}

enum Folded {
    Ignored,
    Eligible,
}

fn fold<'a>(
    slots: &mut HashMap<PrKey, PrSlot<'a>>,
    performance: &'a CanonicalPerformance,
    parents: &Parents<'a>,
    retained: &RetainedContexts<'a>,
    admission: &EventAdmission<'_>,
) -> Result<Folded, CombinedExclusionReason> {
    let Some(candidate) = Candidate::resolve(performance, parents, admission) else {
        return Ok(Folded::Ignored);
    };
    let (key, candidate) = candidate?;
    let integrity = retained.integrity(&candidate, admission.semantics());
    let course_key = key.same_course(candidate.event, admission.semantics());
    if let Some(course_key) = course_key {
        candidate.record(
            slots.entry(course_key).or_insert_with(PrSlot::empty),
            integrity,
            admission.semantics(),
        );
    }
    let entry = slots.entry(key).or_insert_with(PrSlot::empty);
    candidate.record(entry, integrity, admission.semantics());
    Ok(Folded::Eligible)
}

fn close(
    key: PrKey,
    slot: PrSlot<'_>,
    parents: &Parents<'_>,
    scope: Scope,
    semantics: census_domain::model::EventSemantics,
) -> Option<SharedSelection> {
    let candidate = slot.candidate()?;
    let specification = candidate
        .event
        .resolved_specification_with_policy(semantics)
        .ok()?;
    let mut row = make_row(key, candidate, parents, scope, specification, semantics);
    let eligible = slot
        .reports
        .values()
        .filter_map(ReportGroup::eligible)
        .count();
    let representatives = slot
        .reports
        .values()
        .filter_map(ReportGroup::eligible)
        .filter(|value| value.value == candidate.value)
        .map(|value| value.performance.id.clone())
        .collect();
    row.inventory = super::SelectionInventory::from_groups(semantics, eligible, representatives);
    row.supporting_tied_candidate_ids = slot
        .reports
        .values()
        .flat_map(|group| group.supporting_tied_candidates(candidate.value))
        .cloned()
        .collect();
    row.supporting_tied_candidate_ids.sort();
    row.supporting_tied_candidate_ids.dedup();
    row.population = Population {
        marks: slot.marks,
        sources: slot.sources.len(),
    };
    row.conflicts = slot
        .reports
        .into_values()
        .filter_map(ReportGroup::conflict)
        .collect();
    Some(row)
}

fn make_row(
    key: PrKey,
    candidate: &Candidate<'_>,
    parents: &Parents<'_>,
    scope: Scope,
    specification: EventSpecification,
    semantics: census_domain::model::EventSemantics,
) -> SharedSelection {
    SharedSelection {
        key,
        result: candidate.result(),
        meet: candidate.meet(),
        source: candidate.source(scope, specification),
        athlete: candidate.athlete(parents),
        population: Population::default(),
        conflicts: Vec::new(),
        inventory: super::SelectionInventory::from_groups(semantics, 0, Vec::new()),
        supporting_tied_candidate_ids: Vec::new(),
    }
}

fn owner_of<'a>(
    performance: &'a CanonicalPerformance,
    athlete: &'a CanonicalAthlete,
) -> Option<&'a SourceIdentity> {
    performance
        .source_athlete
        .as_ref()
        .or(athlete.source.as_ref())
}

fn resolve_school(team_id: &census_domain::model::TeamId, parents: &Parents<'_>) -> Option<String> {
    parents
        .team(team_id.as_str())
        .and_then(|team| parents.school(team.school.as_str()))
        .map(|school| school.name.clone())
}

fn eligible_measure(
    performance: &CanonicalPerformance,
    kind: &census_domain::model::EventKind,
) -> Option<(Measure, i64)> {
    if kind.is_relay() || !performance.mark_compatible(kind) {
        return None;
    }
    let measure = Measure::of(&performance.mark)?;
    Some((measure, measure.value(&performance.mark)?))
}
