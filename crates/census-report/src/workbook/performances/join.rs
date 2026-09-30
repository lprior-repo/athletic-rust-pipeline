use super::rows::PerformanceRow;
use crate::bests::{mark_text, sport_of, Measure};
use crate::report::Derivation;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    Evidence, Mark, MEET_STATE_UNRESOLVED,
};
use std::collections::HashMap;

pub(super) struct Parents<'a> {
    athletes: Vec<CanonicalAthlete>,
    schools: Vec<CanonicalSchool>,
    meets: Vec<CanonicalMeet>,
    events: Vec<CanonicalEvent>,
    aliases: &'a HashMap<String, String>,
}

impl<'a> Parents<'a> {
    pub(super) fn of(derivation: &Derivation<'a>) -> Self {
        Self {
            athletes: derivation.athletes().to_vec(),
            schools: derivation.schools().to_vec(),
            meets: derivation.meets().to_vec(),
            events: derivation.events().to_vec(),
            aliases: derivation.athlete_aliases(),
        }
    }

    pub(super) fn lookups(&self) -> Lookups<'_> {
        Lookups::of(
            &self.athletes,
            &self.schools,
            &self.meets,
            &self.events,
            self.aliases,
        )
    }
}

pub(super) struct Lookups<'a> {
    athletes: HashMap<&'a str, &'a CanonicalAthlete>,
    schools: HashMap<&'a str, &'a CanonicalSchool>,
    meets: HashMap<&'a str, &'a CanonicalMeet>,
    events: HashMap<&'a str, &'a CanonicalEvent>,
    aliases: &'a HashMap<String, String>,
}

impl<'a> Lookups<'a> {
    fn of(
        athletes: &'a [CanonicalAthlete],
        schools: &'a [CanonicalSchool],
        meets: &'a [CanonicalMeet],
        events: &'a [CanonicalEvent],
        aliases: &'a HashMap<String, String>,
    ) -> Self {
        Self {
            athletes: index(athletes, |row| row.id.as_str()),
            schools: index(schools, |row| row.id.as_str()),
            meets: index(meets, |row| row.id.as_str()),
            events: index(events, |row| row.id.as_str()),
            aliases,
        }
    }

    pub(super) fn school_names(&self) -> Vec<&'a str> {
        let mut names: Vec<&'a str> = self
            .schools
            .values()
            .map(|school| school.name.as_str())
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    pub(super) fn row(&self, performance: &CanonicalPerformance) -> PerformanceRow {
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
            school: athlete.and_then(|row| self.schools.get(row.school.as_str()).copied()),
            meet: self.meets.get(performance.meet.as_str()).copied(),
            event: self.events.get(performance.event.as_str()).copied(),
            source: observed(performance),
        }
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
            .unwrap_or_default()
    }

    fn school_name(&self) -> String {
        self.school.map(|row| row.name.clone()).unwrap_or_default()
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
        self.meet.map(|row| row.name.clone()).unwrap_or_default()
    }

    fn sport(&self) -> String {
        self.event
            .map(|row| sport_of(&row.kind).to_string())
            .unwrap_or_default()
    }

    fn event_label(&self) -> String {
        self.event
            .map(|row| row.kind.stable_key().into_owned())
            .unwrap_or_default()
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
            .unwrap_or_default()
    }

    fn source_url(&self) -> String {
        self.source
            .and_then(|row| row.source.url.clone())
            .unwrap_or_default()
    }
}

fn index<'a, T>(rows: &'a [T], id: impl Fn(&'a T) -> &'a str) -> HashMap<&'a str, &'a T> {
    rows.iter().map(|row| (id(row), row)).collect()
}

fn observed(performance: &CanonicalPerformance) -> Option<&Evidence> {
    performance.evidence.iter().min_by(|left, right| {
        left.source
            .id
            .cmp(&right.source.id)
            .then_with(|| left.source.url.cmp(&right.source.url))
    })
}

fn normalized_mark(mark: &Mark) -> Option<f64> {
    Measure::of(mark)?.normalized_mark(mark)
}
