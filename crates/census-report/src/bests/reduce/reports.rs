use super::super::mark_text;
use super::super::selection::Conflict;
use super::Candidate;
use census_domain::model::CanonicalPerformance;
use std::collections::BTreeSet;

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

pub(super) struct ReportGroup<'a> {
    meet_name: &'a str,
    marks: BTreeSet<String>,
    state: ReportState<'a>,
}

enum ReportState<'a> {
    Uncontested(Candidate<'a>),
    Contested,
}

impl<'a> ReportGroup<'a> {
    pub(super) fn new(candidate: Candidate<'a>) -> Self {
        Self {
            meet_name: candidate.meet.map_or("", |meet| meet.name.as_str()),
            marks: BTreeSet::from([mark_text(&candidate.performance.mark)]),
            state: ReportState::Uncontested(candidate),
        }
    }

    pub(super) fn record(&mut self, candidate: Candidate<'a>) {
        self.marks.insert(mark_text(&candidate.performance.mark));
        if let ReportState::Uncontested(incumbent) = &mut self.state {
            if candidate.value != incumbent.value {
                self.state = ReportState::Contested;
            } else if candidate.wins_over(incumbent) {
                *incumbent = candidate;
            }
        }
    }

    pub(super) fn eligible(&self) -> Option<&Candidate<'a>> {
        match &self.state {
            ReportState::Uncontested(candidate) => Some(candidate),
            ReportState::Contested => None,
        }
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
