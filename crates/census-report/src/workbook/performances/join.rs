use super::rows::PerformanceRow;
use crate::bests::{mark_text, sport_of, Measure};
use crate::report::{
    exclude_out_of_scope, in_run_scope, jurisdiction_of, retain_core, school_state_index,
    ReportResult, Scope,
};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    Evidence, Mark, MEET_STATE_UNRESOLVED,
};
use census_domain::JurisdictionBucket;
use census_store::{Store, Table};
use std::collections::{HashMap, HashSet};

pub(super) struct Parents {
    athletes: Vec<CanonicalAthlete>,
    schools: Vec<CanonicalSchool>,
    meets: Vec<CanonicalMeet>,
    events: Vec<CanonicalEvent>,
}

impl Parents {
    pub(super) fn read(store: &Store, scope: Scope, grad_year: Option<i16>) -> ReportResult<Self> {
        let mut athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
        let mut meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
        let mut events: Vec<CanonicalEvent> = store.scan(Table::Events)?;
        if scope == Scope::Core {
            retain_core(&mut athletes);
            retain_core(&mut meets);
            retain_core(&mut events);
        }
        let mut schools: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
        let outside_schools = exclude_out_of_scope(&mut schools, |school| school.state.into());
        let mut school_state = school_state_index(&schools);
        school_state.extend(school_state_index(&outside_schools));
        athletes.retain(|a| in_run_scope(jurisdiction_of(&school_state, a.school.as_str())));
        meets.retain(|m| in_run_scope(JurisdictionBucket::from(m.state)));
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
    fn of(
        athletes: &'a [CanonicalAthlete],
        schools: &'a [CanonicalSchool],
        meets: &'a [CanonicalMeet],
        events: &'a [CanonicalEvent],
    ) -> Self {
        Self {
            athletes: index(athletes, |row| row.id.as_str()),
            schools: index(schools, |row| row.id.as_str()),
            meets: index(meets, |row| row.id.as_str()),
            events: index(events, |row| row.id.as_str()),
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
        let athlete = self.athletes.get(performance.athlete.as_str()).copied();
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
