mod reports;

use reports::{CompetitionContext, ReportGroup};

use super::key::{should_replace, MarkOrdering, PrKey};
use super::selection::{Population, SharedSelection};
use super::{is_relay, Measure, Options, Parents};
use crate::report::{Derivation, Scope};
use census_domain::model::{
    AthleteId, CanonicalAthlete, CanonicalMeet, CanonicalPerformance, SourceIdentity,
};
use census_domain::{JurisdictionBucket, MeetState};
use std::collections::{BTreeMap, HashMap};

pub fn build_from_dataset(
    dataset: &crate::export::ExportDataset,
    options: &Options,
) -> Vec<SharedSelection> {
    let derivation = Derivation::of(dataset, options.scope, options.grad_year);
    let parents = Parents::of(&derivation);
    let mut slots: HashMap<PrKey, PrSlot> = HashMap::new();
    for performance in derivation.performances() {
        let athlete_id = parents
            .athlete(performance.athlete.as_str())
            .map_or_else(|| performance.athlete.clone(), |athlete| athlete.id.clone());
        fold(&mut slots, performance, &parents, athlete_id);
    }
    super::selection::publish(
        slots
            .into_values()
            .filter_map(|slot| close(slot, &parents, derivation.scope()))
            .collect(),
        options.limit,
    )
}

struct PrSlot<'a> {
    sources: Vec<SourceIdentity>,
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

struct Candidate<'a> {
    key: PrKey,
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
        athlete_id: AthleteId,
    ) -> Option<Self> {
        let athlete = parents.athlete(athlete_id.as_str())?;
        let kind = parents.event(performance.event.as_str())?;
        if is_relay(kind) || !performance.mark_compatible(kind) {
            return None;
        }
        let measure = Measure::of(&performance.mark)?;
        let value = measure.value(&performance.mark)?;
        let meet = parents.meet(performance.meet.as_str());
        let mut key = PrKey::from_performance(performance, kind, meet, measure)?;
        key.athlete_id = athlete_id;
        Some(Self {
            key,
            athlete,
            meet,
            performance,
            measure,
            value,
        })
    }

    fn record(self, entry: &mut PrSlot<'a>) {
        entry.marks = entry.marks.saturating_add(1);
        if let Some(owner) = owner_of(self.performance, self.athlete) {
            if !entry.sources.contains(owner) {
                entry.sources.push(owner.clone());
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
    athlete_id: AthleteId,
) {
    let Some(candidate) = Candidate::resolve(performance, parents, athlete_id) else {
        return;
    };
    let entry = slots
        .entry(candidate.key.clone())
        .or_insert_with(PrSlot::empty);
    candidate.record(entry);
}

fn close(slot: PrSlot<'_>, parents: &Parents<'_>, scope: Scope) -> Option<SharedSelection> {
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
    let mut winner = make_row(candidate, parents, scope);
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

fn make_row(candidate: &Candidate<'_>, parents: &Parents<'_>, scope: Scope) -> SharedSelection {
    let performance = candidate.performance;
    let athlete = candidate.athlete;
    let meet = candidate.meet;
    let school_prov = resolve_school(&performance.team, parents);

    let result_url = scope
        .primary_evidence(performance)
        .and_then(|evidence| evidence.source.url.clone())
        .map_or(Default::default(), core::convert::identity);

    let normalized = candidate.measure.normalized_mark(&performance.mark);

    SharedSelection {
        key: candidate.key.clone(),
        value: candidate.value,
        normalized,
        mark: performance.mark.clone(),
        date: performance.date.clone(),
        meet: meet
            .map(|m| m.name.clone())
            .map_or(Default::default(), core::convert::identity),
        meet_id: performance.meet.clone(),
        meet_state: MeetState::from(meet.and_then(|m| m.state)),
        place: performance.place,
        wind_mps: performance.wind_mps,
        timing: performance.timing,
        result_url,
        performance_id: performance.id.clone(),
        source_athlete: owner_of(performance, athlete)
            .map(|identity| identity.id.clone())
            .map_or(Default::default(), core::convert::identity),
        source_key: performance.source_key.clone(),
        athlete: athlete.canonical_name.clone(),
        gender: athlete.gender,
        grad_year: athlete.grad_year.get(),
        profile_url: athlete.public_profile_urls.first().cloned(),
        school: school_prov,
        athlete_school: athlete.school.as_str().to_string(),
        athlete_state: JurisdictionBucket::from(
            parents
                .school(athlete.school.as_str())
                .and_then(|s| s.state),
        ),
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
