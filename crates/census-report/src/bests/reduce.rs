use super::key::{should_replace, MarkOrdering, PrKey};
use super::selection::{Conflict, Population, SharedSelection};
use super::{is_relay, mark_text, Measure, Options, Parents};
use crate::report::{retain_core_row, Scope};
use census_domain::model::{
    CanonicalAthlete, CanonicalMeet, CanonicalPerformance, Mark, SourceIdentity,
};
use census_domain::{JurisdictionBucket, MeetState};
use census_store::{Store, StoreResult, Table};
use std::collections::HashMap;

pub fn build(store: &Store, options: &Options) -> StoreResult<Vec<SharedSelection>> {
    let snapshot = store.snapshot();
    let parents = Parents::read(&snapshot, options.scope, options.grad_year)?;
    build_with(&snapshot, &parents, options)
}

pub(crate) fn build_with(
    snapshot: &census_store::StoreSnapshot<'_>,
    parents: &Parents,
    options: &Options,
) -> StoreResult<Vec<SharedSelection>> {
    let mut slots: HashMap<PrKey, PrSlot> = HashMap::new();

    snapshot.for_each_merged(Table::Performances, |mut performance| {
        if options.scope == Scope::Core && !retain_core_row(&mut performance) {
            return Ok(());
        }
        fold(&mut slots, &performance, parents, options);
        Ok(())
    })?;

    Ok(super::selection::publish(
        slots.into_values().filter_map(close).collect(),
        options.limit,
    ))
}

pub(crate) struct PrSlot {
    winner: Option<SharedSelection>,
    sources: Vec<SourceIdentity>,
    reports: Vec<(String, String)>,
    marks: usize,
}

impl PrSlot {
    fn empty() -> Self {
        Self {
            winner: None,
            sources: Vec::new(),
            reports: Vec::new(),
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
        parents: &'a Parents,
        options: &Options,
    ) -> Option<Self> {
        let athlete = parents.athlete(performance.athlete.as_str())?;
        if let Some(year) = options.grad_year {
            if athlete.grad_year.get() != year {
                return None;
            }
        }
        let kind = parents.event(performance.event.as_str())?;
        if is_relay(kind) {
            return None;
        }
        let measure = Measure::of(&performance.mark)?;
        let value = measure.value(&performance.mark)?;
        let meet = parents.meet(performance.meet.as_str());
        let key = PrKey::from_performance(performance, kind, meet, measure)?;
        Some(Self {
            key,
            athlete,
            meet,
            performance,
            measure,
            value,
        })
    }

    fn record(&self, entry: &mut PrSlot, parents: &Parents) {
        entry.marks = entry.marks.saturating_add(1);
        if !entry.sources.contains(&self.performance.source_athlete) {
            entry.sources.push(self.performance.source_athlete.clone());
        }
        let meet_name = self.meet.map(|meet| meet.name.clone()).unwrap_or_default();
        entry
            .reports
            .push((meet_name, format_mark(&self.performance.mark)));
        if self.wins_over(entry) {
            entry.winner = Some(make_row(
                self.key.clone(),
                self.athlete,
                self.meet,
                self.performance,
                self.value,
                self.measure,
                parents,
            ));
        }
    }

    fn wins_over(&self, entry: &PrSlot) -> bool {
        entry
            .winner
            .as_ref()
            .map(|winner| {
                should_replace(
                    self.value,
                    winner.value,
                    MarkOrdering::new(
                        &self.performance.date,
                        self.performance.meet.as_str(),
                        self.performance.id.as_str(),
                    ),
                    MarkOrdering::new(
                        &winner.date,
                        winner.meet_id.as_str(),
                        winner.performance_id.as_str(),
                    ),
                    move |candidate, incumbent| self.measure.better(candidate, incumbent),
                )
            })
            .unwrap_or(true)
    }
}

fn fold(
    slots: &mut HashMap<PrKey, PrSlot>,
    performance: &CanonicalPerformance,
    parents: &Parents,
    options: &Options,
) {
    let Some(candidate) = Candidate::resolve(performance, parents, options) else {
        return;
    };
    let entry = slots
        .entry(candidate.key.clone())
        .or_insert_with(PrSlot::empty);
    candidate.record(entry, parents);
}

fn close(slot: PrSlot) -> Option<SharedSelection> {
    let mut winner = slot.winner?;
    let sources_count = slot.sources.len();
    let mut conflicts = Vec::new();

    let mut meet_marks: HashMap<String, Vec<String>> = HashMap::new();
    for (meet, mark) in &slot.reports {
        meet_marks
            .entry(meet.clone())
            .or_default()
            .push(mark.clone());
    }
    for (meet, marks) in meet_marks {
        if marks.len() > 1 {
            let unique: Vec<String> = marks.into_iter().collect();
            if unique.len() > 1 {
                conflicts.push(Conflict {
                    meet,
                    marks: unique,
                });
            }
        }
    }

    winner.population = Population {
        marks: slot.marks,
        sources: sources_count,
    };
    winner.conflicts = conflicts;
    Some(winner)
}

fn format_mark(mark: &Mark) -> String {
    mark_text(mark)
}

fn make_row(
    key: PrKey,
    athlete: &CanonicalAthlete,
    meet: Option<&CanonicalMeet>,
    performance: &CanonicalPerformance,
    value: i64,
    measure: Measure,
    parents: &Parents,
) -> SharedSelection {
    let school_prov = resolve_school(&performance.team, parents);

    let result_url = performance
        .evidence
        .iter()
        .find_map(|e| e.source.url.clone())
        .unwrap_or_default();

    let normalized = measure.normalized_mark(&performance.mark);

    SharedSelection {
        key,
        value,
        normalized,
        mark: performance.mark.clone(),
        date: performance.date.clone(),
        meet: meet.map(|m| m.name.clone()).unwrap_or_default(),
        meet_id: performance.meet.clone(),
        meet_state: MeetState::from(meet.and_then(|m| m.state)),
        place: performance.place,
        wind_mps: performance.wind_mps,
        timing: performance.timing,
        result_url,
        performance_id: performance.id.clone(),
        source_athlete: performance.source_athlete.id.clone(),
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

fn resolve_school(team_id: &census_domain::model::TeamId, parents: &Parents) -> Option<String> {
    parents
        .team(team_id.as_str())
        .and_then(|team| parents.school(team.school.as_str()))
        .map(|school| school.name.clone())
}
