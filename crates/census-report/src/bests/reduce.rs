mod combined;
mod reports;
mod row;
mod support;

use reports::{CompetitionContext, ContextIntegrity, ReportGroup, RetainedContexts};

use super::selection::{Population, SharedSelection};
use super::{Options, Parents};
use crate::report::{Derivation, EventAdmission, ReportResult, Scope};
use census_domain::model::performance_comparison::Measure;
use census_domain::model::performance_comparison::{should_replace, MarkOrdering, PrKey};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, EventSpecification,
    SourceIdentity,
};
use std::collections::{BTreeMap, HashMap};

pub fn build_from_dataset(
    dataset: &crate::export::ExportDataset,
    options: &Options,
) -> ReportResult<Vec<SharedSelection>> {
    let admission = EventAdmission::of(dataset)?;
    build_admitted(&admission, options)
}

pub(crate) fn build_admitted(
    admission: &EventAdmission<'_>,
    options: &Options,
) -> ReportResult<Vec<SharedSelection>> {
    let derivation = Derivation::of(
        admission.dataset(),
        options.scope,
        options.grad_year,
        &options.jurisdictions,
    );
    support::admit(derivation.performances())?;
    let parents = Parents::of(&derivation);
    let retained = RetainedContexts::of(derivation.performances(), &parents, admission);
    let mut slots: HashMap<PrKey, PrSlot<'_>> = HashMap::new();
    derivation
        .performances()
        .iter()
        .copied()
        .for_each(|performance| {
            fold(&mut slots, performance, &parents, &retained, admission);
        });
    Ok(super::selection::publish(
        slots
            .into_iter()
            .filter_map(|(key, slot)| {
                close(
                    key,
                    slot,
                    &parents,
                    derivation.scope(),
                    admission.semantics(),
                )
            })
            .collect(),
        options.limit,
    ))
}

struct PrSlot<'a> {
    sources: Vec<&'a SourceIdentity>,
    reports: BTreeMap<CompetitionContext<'a>, ReportGroup<'a>>,
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
    ) -> Option<(PrKey, Self)> {
        let athlete = parents.athlete(performance.athlete.as_str())?;
        let kind = admission.included_kind(&performance.event)?;
        let (measure, value) = eligible_measure(performance, kind)?;
        let meet = parents.meet(performance.meet.as_str());
        let event = parents.canonical_event(performance.event.as_str())?;
        if admission.semantics().requires_completed_combined()
            && matches!(
                kind,
                census_domain::model::EventKind::Pentathlon
                    | census_domain::model::EventKind::Heptathlon
                    | census_domain::model::EventKind::Decathlon
            )
            && performance.combined_eligibility(
                &event
                    .resolved_specification_with_policy(admission.semantics())
                    .ok()?,
            ) != census_domain::model::CombinedEligibility::Eligible
        {
            return None;
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

    fn record(self, entry: &mut PrSlot<'a>, integrity: ContextIntegrity) {
        entry.marks = entry.marks.saturating_add(1);
        if let Some(owner) = owner_of(self.performance, self.athlete) {
            if !entry.sources.contains(&owner) {
                entry.sources.push(owner);
            }
        }
        let context = CompetitionContext::of(self.performance);
        match entry.reports.entry(context) {
            std::collections::btree_map::Entry::Vacant(vacant) => {
                vacant.insert(ReportGroup::new(self, integrity));
            }
            std::collections::btree_map::Entry::Occupied(mut occupied) => {
                occupied.get_mut().record(self, integrity);
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

fn fold<'a>(
    slots: &mut HashMap<PrKey, PrSlot<'a>>,
    performance: &'a CanonicalPerformance,
    parents: &Parents<'a>,
    retained: &RetainedContexts<'a>,
    admission: &EventAdmission<'_>,
) {
    let Some((key, candidate)) = Candidate::resolve(performance, parents, admission) else {
        return;
    };
    let integrity = retained.integrity(&candidate);
    let course_key = key.same_course(candidate.event, admission.semantics());
    if let Some(course_key) = course_key {
        candidate.record(
            slots.entry(course_key).or_insert_with(PrSlot::empty),
            integrity,
        );
    }
    let entry = slots.entry(key).or_insert_with(PrSlot::empty);
    candidate.record(entry, integrity);
}

fn close(
    key: PrKey,
    slot: PrSlot<'_>,
    parents: &Parents<'_>,
    scope: Scope,
    semantics: census_domain::model::EventSemantics,
) -> Option<SharedSelection> {
    let candidate = slot
        .reports
        .values()
        .filter_map(ReportGroup::eligible)
        .reduce(|incumbent, candidate| {
            if candidate.wins_over(incumbent) {
                candidate
            } else {
                incumbent
            }
        })?;
    let specification = candidate
        .event
        .resolved_specification_with_policy(semantics)
        .ok()?;
    let mut row = make_row(key, candidate, parents, scope, specification);
    row.eligible_candidate_count = slot
        .reports
        .values()
        .filter_map(ReportGroup::eligible)
        .count();
    row.tied_performance_ids = slot
        .reports
        .values()
        .filter_map(ReportGroup::eligible)
        .filter(|value| value.value == candidate.value)
        .map(|value| value.performance.id.clone())
        .collect();
    row.tied_performance_ids.sort();
    row.tied_performance_ids.dedup();
    row.tied_performance_count = row.tied_performance_ids.len();
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
) -> SharedSelection {
    SharedSelection {
        key,
        result: candidate.result(),
        meet: candidate.meet(),
        source: candidate.source(scope, specification),
        athlete: candidate.athlete(parents),
        population: Population::default(),
        conflicts: Vec::new(),
        eligible_candidate_count: 0,
        tied_performance_count: 0,
        tied_performance_ids: Vec::new(),
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
