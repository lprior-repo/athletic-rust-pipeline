mod reports;
mod row;

use reports::{CompetitionContext, ReportGroup};

use super::key::{should_replace, MarkOrdering, PrKey};
use super::selection::{Population, SharedSelection};
use super::{is_relay, Measure, Options, Parents};
use crate::report::{Derivation, Scope};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, EventSpecification,
    SourceIdentity,
};
use std::collections::{BTreeMap, HashMap};

pub fn build_from_dataset(
    dataset: &crate::export::ExportDataset,
    options: &Options,
) -> Vec<SharedSelection> {
    let derivation = Derivation::of(dataset, options.scope, options.grad_year);
    let parents = Parents::of(&derivation);
    let mut slots: HashMap<PrKey, PrSlot<'_>> = HashMap::new();
    derivation
        .performances()
        .iter()
        .copied()
        .for_each(|performance| {
            fold(&mut slots, performance, &parents);
        });
    super::selection::publish(
        slots
            .into_iter()
            .filter_map(|(key, slot)| close(key, slot, &parents, derivation.scope()))
            .collect(),
        options.limit,
    )
}

struct PrSlot<'a> {
    sources: Vec<&'a SourceIdentity>,
    reports: BTreeMap<CompetitionContext<'a>, ReportGroup<'a>>,
    marks: usize,
}

impl PrSlot<'_> {
    fn empty() -> Self {
        Self {
            sources: Vec::new(),
            reports: BTreeMap::new(),
            marks: 0,
        }
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
    ) -> Option<(PrKey, Self)> {
        let athlete = parents.athlete(performance.athlete.as_str())?;
        let (measure, value) = eligible_measure(performance, parents)?;
        let meet = parents.meet(performance.meet.as_str());
        let event = parents.canonical_event(performance.event.as_str())?;
        let key = PrKey::from_athlete(performance, event, meet, measure, &athlete.id)?;
        Some((
            key,
            Self {
                event,
                athlete,
                meet,
                performance,
                measure,
                value,
            },
        ))
    }

    fn record(self, entry: &mut PrSlot<'a>) {
        entry.marks = entry.marks.saturating_add(1);
        if let Some(owner) = owner_of(self.performance, self.athlete) {
            if !entry.sources.contains(&owner) {
                entry.sources.push(owner);
            }
        }
        let context = CompetitionContext::of(self.performance);
        match entry.reports.entry(context) {
            std::collections::btree_map::Entry::Vacant(vacant) => {
                vacant.insert(ReportGroup::new(self));
            }
            std::collections::btree_map::Entry::Occupied(mut occupied) => {
                occupied.get_mut().record(self);
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
        MarkOrdering::new(
            &self.performance.date,
            self.performance.meet.as_str(),
            self.performance.id.as_str(),
        )
    }
}

fn fold<'a>(
    slots: &mut HashMap<PrKey, PrSlot<'a>>,
    performance: &'a CanonicalPerformance,
    parents: &Parents<'a>,
) {
    let Some((key, candidate)) = Candidate::resolve(performance, parents) else {
        return;
    };
    let course_key = key.same_course(candidate.event);
    if let Some(course_key) = course_key {
        candidate.record(slots.entry(course_key).or_insert_with(PrSlot::empty));
    }
    let entry = slots.entry(key).or_insert_with(PrSlot::empty);
    candidate.record(entry);
}

fn close(
    key: PrKey,
    slot: PrSlot<'_>,
    parents: &Parents<'_>,
    scope: Scope,
) -> Option<SharedSelection> {
    let candidate = winning_candidate(&slot)?;
    let specification = candidate.event.resolved_specification().ok()?;
    let mut winner = make_row(key, candidate, parents, scope, specification);
    let sources_count = slot.sources.len();
    let conflicts = slot
        .reports
        .into_values()
        .filter_map(ReportGroup::conflict)
        .collect();

    winner.population = Population {
        marks: slot.marks,
        sources: sources_count,
    };
    winner.conflicts = conflicts;
    Some(winner)
}

fn winning_candidate<'a, 'slot>(slot: &'slot PrSlot<'a>) -> Option<&'slot Candidate<'a>> {
    slot.reports
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

fn make_row(
    key: PrKey,
    candidate: &Candidate<'_>,
    parents: &Parents<'_>,
    scope: Scope,
    specification: EventSpecification,
) -> SharedSelection {
    SharedSelection {
        key,
        result: candidate.result(),
        meet: candidate.meet(),
        source: candidate.source(scope, specification),
        athlete: candidate.athlete(parents),
        population: Population::default(),
        conflicts: Vec::new(),
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
    parents: &Parents<'_>,
) -> Option<(Measure, i64)> {
    let kind = parents.event(performance.event.as_str())?;
    if is_relay(kind) || !performance.mark_compatible(kind) {
        return None;
    }
    let measure = Measure::of(&performance.mark)?;
    Some((measure, measure.value(&performance.mark)?))
}
