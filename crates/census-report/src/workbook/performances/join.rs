use super::rows::PerformanceRow;
use crate::bests::{mark_text, sport_of, Measure};
use crate::report::{Derivation, Scope};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, Evidence, Mark, MEET_STATE_UNRESOLVED,
};
use std::collections::HashMap;

pub struct PerformanceProjection<'a> {
    athletes: HashMap<&'a str, &'a CanonicalAthlete>,
    schools: HashMap<&'a str, &'a CanonicalSchool>,
    teams: HashMap<&'a str, &'a CanonicalTeam>,
    meets: HashMap<&'a str, &'a CanonicalMeet>,
    events: HashMap<&'a str, &'a CanonicalEvent>,
    aliases: &'a HashMap<String, String>,
    scope: Scope,
}

impl<'a> PerformanceProjection<'a> {
    pub fn of(derivation: &'a Derivation<'_>) -> Self {
        Self {
            athletes: index(derivation.athletes().iter(), |row| row.id.as_str()),
            schools: index(derivation.schools().iter(), |row| row.id.as_str()),
            teams: index(derivation.dataset().teams.values(), |row| row.id.as_str()),
            meets: index(derivation.meets().iter(), |row| row.id.as_str()),
            events: index(derivation.events().iter(), |row| row.id.as_str()),
            aliases: derivation.athlete_aliases(),
            scope: derivation.scope(),
        }
    }

    pub(super) fn school_names(&self) -> Vec<&'a str> {
        let mut names: Vec<&str> = self
            .schools
            .values()
            .map(|school| school.name.as_str())
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    pub fn row(&self, performance: &CanonicalPerformance) -> PerformanceRow {
        let joins = self.joins(performance);
        PerformanceRow {
            id: performance.id.as_str().to_string(),
            athlete_id: performance.athlete.as_str().to_string(),
            athlete: joins.athlete_name(),
            school: joins.school_name(),
            grad_year: joins.grad_year(),
            meet_id: performance.meet.as_str().to_string(),
            meet: joins.meet_name(),
            date: performance.date.clone(),
            state: joins.state(),
            sport: joins.sport(),
            event: joins.event_label(),
            mark: mark_text(&performance.mark),
            normalized: normalized_mark(&performance.mark),
            timing: performance.timing.map(|method| format!("{method:?}")),
            wind_mps: performance.wind_mps,
            round: joins.round_of(performance),
            place: performance.place,
            source: joins.source_id(),
            source_result: performance.source_key.clone(),
            source_url: joins.source_url(),
        }
    }

    fn joins<'b>(&self, performance: &'b CanonicalPerformance) -> Joins<'a, 'b> {
        let subject = performance.athlete.as_str();
        let canonical = self.aliases.get(subject).map_or(subject, String::as_str);
        let athlete = self.athletes.get(canonical).copied();
        Joins {
            athlete,
            school: self.result_school(performance),
            meet: self.meets.get(performance.meet.as_str()).copied(),
            event: self.events.get(performance.event.as_str()).copied(),
            source: self.scope.primary_evidence(performance),
        }
    }

    fn result_school(&self, performance: &CanonicalPerformance) -> Option<&'a CanonicalSchool> {
        let team = self.teams.get(performance.team.as_str())?;
        self.schools.get(team.school.as_str()).copied()
    }
}

struct Joins<'a, 'b> {
    athlete: Option<&'a CanonicalAthlete>,
    school: Option<&'a CanonicalSchool>,
    meet: Option<&'a CanonicalMeet>,
    event: Option<&'a CanonicalEvent>,
    source: Option<&'b Evidence>,
}

impl Joins<'_, '_> {
    fn athlete_name(&self) -> String {
        self.athlete
            .map(|row| row.canonical_name.clone())
            .map_or(Default::default(), core::convert::identity)
    }

    fn school_name(&self) -> String {
        self.school
            .map(|row| row.name.clone())
            .map_or(Default::default(), core::convert::identity)
    }

    fn grad_year(&self) -> Option<i16> {
        self.athlete.map(|row| row.grad_year.get())
    }

    fn state(&self) -> Option<String> {
        self.meet.map(|row| {
            row.state.map_or_else(
                || MEET_STATE_UNRESOLVED.to_string(),
                |state| state.code().to_string(),
            )
        })
    }

    fn meet_name(&self) -> String {
        self.meet
            .map(|row| row.name.clone())
            .map_or(Default::default(), core::convert::identity)
    }

    fn sport(&self) -> String {
        self.event
            .map(|row| sport_of(&row.kind).to_string())
            .map_or(Default::default(), core::convert::identity)
    }

    fn event_label(&self) -> String {
        self.event
            .map(|row| row.kind.stable_key().into_owned())
            .map_or(Default::default(), core::convert::identity)
    }

    fn round_of(&self, performance: &CanonicalPerformance) -> Option<String> {
        performance
            .round
            .clone()
            .or_else(|| self.event.and_then(|row| row.round.clone()))
    }

    fn source_id(&self) -> String {
        self.source
            .map(|row| row.source.id.clone())
            .map_or(Default::default(), core::convert::identity)
    }

    fn source_url(&self) -> String {
        self.source
            .and_then(|row| row.source.url.clone())
            .map_or(Default::default(), core::convert::identity)
    }
}

fn index<'a, T>(
    rows: impl Iterator<Item = &'a T>,
    id: impl Fn(&'a T) -> &'a str,
) -> HashMap<&'a str, &'a T> {
    rows.map(|row| (id(row), row)).collect()
}

fn normalized_mark(mark: &Mark) -> Option<f64> {
    Measure::of(mark)?.normalized_mark(mark)
}
