use super::rows::PerformanceRow;
use crate::bests::{mark_text, sport_of, Measure};
use crate::report::{ReportResult, Scope};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    Evidence, Mark, MEET_STATE_UNRESOLVED,
};
use std::collections::{HashMap, HashSet};

pub(super) struct Parents {
    athletes: Vec<CanonicalAthlete>,
    schools: Vec<CanonicalSchool>,
    meets: Vec<CanonicalMeet>,
    events: Vec<CanonicalEvent>,
}

impl Parents {
    pub(super) fn from_shared(
        athletes: &[CanonicalAthlete],
        schools: &[CanonicalSchool],
        meets: &[CanonicalMeet],
        events: &[CanonicalEvent],
    ) -> Self {
        Self {
            athletes: athletes.to_vec(),
            schools: schools.to_vec(),
            meets: meets.to_vec(),
            events: events.to_vec(),
        }
    }

    pub(super) fn read(store: &census_store::Store, scope: Scope, grad_year: Option<i16>) -> ReportResult<Self> {
        let mut athletes: Vec<CanonicalAthlete> = store.scan(census_store::Table::Athletes)?;
        let mut meets: Vec<CanonicalMeet> = store.scan(census_store::Table::Meets)?;
        let mut events: Vec<CanonicalEvent> = store.scan(census_store::Table::Events)?;
        if scope == Scope::Core {
            crate::report::retain_core(&mut athletes);
            crate::report::retain_core(&mut meets);
            crate::report::retain_core(&mut events);
        }
        let mut schools: Vec<CanonicalSchool> = store.scan(census_store::Table::Schools)?;
        let outside_schools = crate::report::exclude_out_of_scope(&mut schools, |school| school.state.into());
        let mut school_state = crate::report::school_state_index(&schools);
        school_state.extend(crate::report::school_state_index(&outside_schools));
        athletes.retain(|a| crate::report::in_run_scope(crate::report::jurisdiction_of(&school_state, a.school.as_str())));
        meets.retain(|m| crate::report::in_run_scope(census_domain::JurisdictionBucket::from(m.state)));
        if let Some(year) = grad_year {
            athletes.retain(|a| a.grad_year.get() == year);
        }
        Ok(Self {
            athletes,
            schools,
            meets,
            events,
        })
    }

    pub(super) fn lookups(&self) -> Lookups<'_> {
        Lookups::of(&self.athletes, &self.schools, &self.meets, &self.events)
    }

    pub(super) fn cohort_set(&self) -> HashSet<&str> {
        self.athletes.iter().map(|a| a.id.as_str()).collect()
    }
}

pub(super) struct Lookups<'a> {
    athletes: HashMap<&'a str, &'a CanonicalAthlete>,
    schools: HashMap<&'a str, &'a CanonicalSchool>,
    meets: HashMap<&'a str, &'a CanonicalMeet>,
    events: HashMap<&'a str, &'a CanonicalEvent>,
}

impl<'a> Lookups<'a> {
    pub(super) fn of(
        athletes: &'a [CanonicalAthlete],
        schools: &'a [CanonicalSchool],
        meets: &'a [CanonicalMeet],
        events: &'a [CanonicalEvent],
    ) -> Self {
        Self {
            athletes: index(athletes, |a| a.id.as_str()),
            schools: index(schools, |s| s.id.as_str()),
            meets: index(meets, |m| m.id.as_str()),
            events: index(events, |e| e.id.as_str()),
        }
    }

    pub(super) fn athlete(&self, id: &str) -> Option<&CanonicalAthlete> {
        self.athletes.get(id).copied()
    }

    pub(super) fn school(&self, id: &str) -> Option<&CanonicalSchool> {
        self.schools.get(id).copied()
    }

    pub(super) fn meet(&self, id: &str) -> Option<&CanonicalMeet> {
        self.meets.get(id).copied()
    }

    pub(super) fn event(&self, id: &str) -> Option<&CanonicalEvent> {
        self.events.get(id).copied()
    }

    pub(super) fn athlete_name(&self, id: &str) -> String {
        self.athlete(id)
            .map(|a| a.display_name().clone())
            .unwrap_or_else(|| id.to_string())
    }

    pub(super) fn athlete_school(&self, id: &str) -> String {
        let Some(athlete) = self.athlete(id) else {
            return String::new();
        };
        self.school_name(&athlete.school)
    }

    pub(super) fn athlete_grad_year(&self, id: &str) -> Option<i16> {
        self.athlete(id).map(|a| a.grad_year.get())
    }

    pub(super) fn school_name(&self, id: &str) -> String {
        self.school(id)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| id.to_string())
    }

    pub(super) fn school_state(&self, id: &str) -> String {
        self.school(id)
            .map(|s| s.state.clone())
            .unwrap_or_default()
    }

    pub(super) fn school_city(&self, id: &str) -> String {
        self.school(id)
            .map(|s| s.city.clone())
            .unwrap_or_default()
    }

    pub(super) fn meet_name(&self, id: &str) -> String {
        self.meet(id)
            .map(|m| m.name.clone())
            .unwrap_or_else(|| id.to_string())
    }

    pub(super) fn meet_location(&self, id: &str) -> String {
        self.meet(id)
            .map(|m| m.location.clone())
            .unwrap_or_default()
    }

    pub(super) fn event_name(&self, id: &str) -> String {
        self.event(id)
            .map(|e| e.name.clone())
            .unwrap_or_else(|| id.to_string())
    }

    pub(super) fn event_sport(&self, id: &str) -> String {
        self.event(id)
            .map(|e| e.sport.clone())
            .unwrap_or_default()
    }
}

fn row(performance: &CanonicalPerformance, lookups: &Lookups<'_>) -> PerformanceRow {
    let athlete_name = lookups.athlete_name(&performance.athlete);
    let athlete_school = lookups.athlete_school(&performance.athlete);
    let athlete_grad = lookups.athlete_grad_year(&performance.athlete);
    let meet_name = lookups.meet_name(&performance.meet);
    let meet_location = lookups.meet_location(&performance.meet);
    let event_name = lookups.event_name(&performance.event);
    let event_sport = lookups.event_sport(&performance.event);
    let observed = observed(performance);
    let normalized = observed
        .and_then(|e| e.mark.as_ref())
        .and_then(|mark| normalized_mark(mark));
    PerformanceRow {
        performance_id: performance.id.as_str().to_string(),
        athlete_name,
        athlete_school,
        athlete_grad,
        meet_name,
        meet_location,
        meet_date: performance.meet_date.clone(),
        event_name,
        event_sport,
        event_distance: performance.event_distance.clone(),
        mark_text: observed
            .and_then(|e| e.mark.as_ref())
            .map(|mark| mark_text(mark))
            .unwrap_or_default(),
        normalized,
        wind: performance.wind.clone(),
        timing: performance.timing.clone(),
        place: performance.place.clone(),
        heat: performance.heat.clone(),
        round: performance.round.clone(),
        attempt: performance.attempt.clone(),
        source: performance.source_id.clone(),
        evidence_id: performance.evidence_id.clone(),
        relay_team: performance.relay_team.clone(),
        individual_split: performance.individual_split,
    }
}

fn index<'a, T>(rows: &'a [T], id: impl Fn(&'a T) -> &'a str) -> HashMap<&'a str, &'a T> {
    rows.iter().map(|row| (id(row), row)).collect()
}

fn observed(performance: &CanonicalPerformance) -> Option<&Evidence> {
    performance
        .evidence
        .iter()
        .find(|e| e.kind == "observed")
}

fn normalized_mark(mark: &Mark) -> Option<f64> {
    Measure::of(mark)?.normalized_mark(mark)
}