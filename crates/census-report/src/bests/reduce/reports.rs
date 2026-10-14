use super::super::mark_text;
use super::super::selection::Conflict;
use super::{Candidate, Parents};
use crate::report::EventAdmission;
use census_domain::model::performance_comparison::Measure;
use census_domain::model::{CanonicalPerformance, PerformanceId};
use std::collections::{BTreeMap, BTreeSet};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct CompetitionContext<'a> {
    meet: &'a str,
    date: &'a str,
    event: &'a str,
    round: Option<&'a str>,
    heat: Option<&'a str>,
}

impl<'a> CompetitionContext<'a> {
    pub(super) fn of(performance: &'a CanonicalPerformance) -> Self {
        Self {
            meet: performance.meet.as_str(),
            date: &performance.date,
            event: performance.event.as_str(),
            round: performance.round.as_deref(),
            heat: performance.heat.as_deref(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ContextIntegrity {
    Clear,
    RetainedConflict,
}

pub(super) struct RetainedContexts<'a>(BTreeSet<(&'a str, CompetitionContext<'a>)>);

impl<'a> RetainedContexts<'a> {
    pub(super) fn of(
        performances: &[&'a CanonicalPerformance],
        parents: &Parents<'a>,
        admission: &EventAdmission<'_>,
    ) -> Self {
        Self(
            performances
                .iter()
                .copied()
                .filter(|performance| {
                    !performance.retained_conflicts.is_empty()
                        || super::combined::explicit_terminal(performance, admission)
                })
                .filter_map(|performance| {
                    let athlete = parents.athlete(performance.athlete.as_str())?;
                    Some((athlete.id.as_str(), CompetitionContext::of(performance)))
                })
                .collect(),
        )
    }

    pub(super) fn integrity(&self, candidate: &Candidate<'a>) -> ContextIntegrity {
        if self.0.contains(&(
            candidate.athlete.id.as_str(),
            CompetitionContext::of(candidate.performance),
        )) {
            ContextIntegrity::RetainedConflict
        } else {
            ContextIntegrity::Clear
        }
    }
}

pub(super) struct ReportGroup<'a> {
    meet_name: &'a str,
    marks: BTreeSet<String>,
    state: ReportState<'a>,
    candidates: BTreeMap<&'a PerformanceId, i64>,
}

enum ReportState<'a> {
    Uncontested(Candidate<'a>),
    Contested,
}

impl<'a> ReportGroup<'a> {
    pub(super) fn new(candidate: Candidate<'a>, integrity: ContextIntegrity) -> Self {
        Self {
            meet_name: candidate.meet.map_or("", |meet| meet.name.as_str()),
            marks: BTreeSet::from([mark_text(&candidate.performance.mark)]),
            state: match integrity {
                ContextIntegrity::Clear => ReportState::Uncontested(candidate),
                ContextIntegrity::RetainedConflict => ReportState::Contested,
            },
            candidates: match integrity {
                ContextIntegrity::Clear => {
                    BTreeMap::from([(&candidate.performance.id, candidate.value)])
                }
                ContextIntegrity::RetainedConflict => BTreeMap::new(),
            },
        }
    }

    pub(super) fn record(&mut self, candidate: Candidate<'a>, integrity: ContextIntegrity) {
        self.marks.insert(mark_text(&candidate.performance.mark));
        if integrity == ContextIntegrity::RetainedConflict {
            self.state = ReportState::Contested;
        }
        let ReportState::Uncontested(incumbent) = &mut self.state else {
            self.candidates.clear();
            return;
        };
        if observed_value(&candidate) != observed_value(incumbent) {
            self.state = ReportState::Contested;
            self.candidates.clear();
            return;
        }
        self.candidates
            .entry(&candidate.performance.id)
            .and_modify(|value| {
                if candidate.measure.better(candidate.value, *value) {
                    *value = candidate.value;
                }
            })
            .or_insert(candidate.value);
        if candidate.wins_over(incumbent) {
            *incumbent = candidate;
        }
    }

    pub(super) fn eligible(&self) -> Option<&Candidate<'a>> {
        match &self.state {
            ReportState::Uncontested(candidate) => Some(candidate),
            ReportState::Contested => None,
        }
    }

    pub(super) fn supporting_tied_candidates(
        &self,
        value: i64,
    ) -> impl Iterator<Item = &'a PerformanceId> + '_ {
        let eligible = self.eligible().is_some();
        self.candidates
            .iter()
            .filter_map(move |(id, candidate_value)| {
                (eligible && *candidate_value == value).then_some(*id)
            })
    }

    pub(super) fn conflict(self) -> Option<Conflict> {
        match self.state {
            ReportState::Uncontested(_) => None,
            ReportState::Contested => Some(Conflict {
                meet: self.meet_name.to_string(),
                marks: self.marks.into_iter().collect(),
            }),
        }
    }
}

fn observed_value(candidate: &Candidate<'_>) -> i64 {
    candidate
        .performance
        .adjustment
        .as_ref()
        .and_then(|adjustment| adjustment.original_mark.as_ref())
        .and_then(|mark| Measure::of(mark).and_then(|measure| measure.value(mark)))
        .map_or(candidate.value, core::convert::identity)
}
