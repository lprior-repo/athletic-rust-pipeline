use super::rows::PerformanceRow;
use crate::bests::{mark_text, sport_of, Measure};
use crate::report::{ReportResult, Scope};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
};
use std::collections::{HashMap, HashSet};

pub(super) struct Parents {
    athletes: Vec<CanonicalAthlete>,
    schools: Vec<CanonicalSchool>,
    meets: Vec<CanonicalMeet>,
    events: Vec<CanonicalEvent>,
}

impl Parents {
    pub(super) fn read(
        store: &census_store::Store,
        scope: Scope,
        grad_year: Option<i16>,
    ) -> ReportResult<Self> {
        let mut athletes: Vec<CanonicalAthlete> = store.scan(census_store::Table::Athletes)?;
        let mut meets: Vec<CanonicalMeet> = store.scan(census_store::Table::Meets)?;
        let mut events: Vec<CanonicalEvent> = store.scan(census_store::Table::Events)?;
        if scope == Scope::Core {
            crate::report::retain_core(&mut athletes);
            crate::report::retain_core(&mut meets);
            crate::report::retain_core(&mut events);
        }
        let mut schools: Vec<CanonicalSchool> = store.scan(census_store::Table::Schools)?;
        let outside_schools =
            crate::report::exclude_out_of_scope(&mut schools, |school| school.state.into());
        let mut school_state = crate::report::school_state_index(&schools);
        school_state.extend(crate::report::school_state_index(&outside_schools));
        athletes.retain(|a| {
            crate::report::in_run_scope(crate::report::jurisdiction_of(
                &school_state,
                a.school.as_str(),
            ))
        });
        meets.retain(|m| {
            crate::report::in_run_scope(census_domain::JurisdictionBucket::from(m.state))
        });
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
            .map(|a| a.canonical_name.clone())
            .unwrap_or_default()
    }

    pub(super) fn athlete_school(&self, id: &str) -> String {
        let Some(athlete) = self.athlete(id) else {
            return String::new();
        };
        self.school_name(athlete.school.as_str())
    }

    pub(super) fn athlete_grad_year(&self, id: &str) -> Option<i16> {
        self.athlete(id).map(|a| a.grad_year.get())
    }

    pub(super) fn school_name(&self, id: &str) -> String {
        self.school(id)
            .map(|s| s.name.clone())
            .unwrap_or_else(|| id.to_string())
    }

    pub(super) fn school_state(&self, id: &str) -> Option<String> {
        self.school(id)
            .and_then(|s| s.state.as_ref())
            .map(|j| j.code().to_string())
    }

    pub(super) fn athlete_school_state(&self, id: &str) -> Option<String> {
        self.athlete(id)
            .map(|athlete| athlete.school.as_str())
            .and_then(|school| self.school_state(school))
    }

    pub(super) fn school_names(&self) -> Vec<&'a str> {
        self.schools
            .values()
            .copied()
            .map(|s| s.name.as_str())
            .collect()
    }

    pub(super) fn meet_name(&self, id: &str) -> String {
        self.meet(id).map(|m| m.name.clone()).unwrap_or_default()
    }

    pub(super) fn event_name(&self, id: &str) -> String {
        self.event(id)
            .map(|e| e.kind.stable_key().to_string())
            .unwrap_or_default()
    }

    pub(super) fn event_sport(&self, id: &str) -> String {
        self.event(id)
            .map(|e| sport_of(&e.kind).to_string())
            .unwrap_or_default()
    }

    pub(super) fn event_round(&self, id: &str) -> Option<String> {
        self.event(id).and_then(|e| e.round.clone())
    }

    pub(super) fn row(&self, performance: &CanonicalPerformance) -> PerformanceRow {
        let athlete_name = self.athlete_name(performance.athlete.as_str());
        let athlete_school = self.athlete_school(performance.athlete.as_str());
        let athlete_grad = self.athlete_grad_year(performance.athlete.as_str());
        let meet_name = self.meet_name(performance.meet.as_str());
        let event_name = self.event_name(performance.event.as_str());
        let event_sport = self.event_sport(performance.event.as_str());
        let event_round = self.event_round(performance.event.as_str());
        let school_state = self.athlete_school_state(performance.athlete.as_str());

        let source = performance
            .evidence
            .first()
            .map(|e| e.source.id.clone())
            .unwrap_or_default();
        let source_url = performance
            .evidence
            .first()
            .and_then(|e| e.source.url.clone())
            .unwrap_or_default();

        let normalized = Measure::of(&performance.mark)
            .and_then(|measure| measure.normalized_mark(&performance.mark));

        PerformanceRow {
            id: performance.id.as_str().to_string(),
            athlete_id: performance.athlete.as_str().to_string(),
            athlete: athlete_name,
            school: athlete_school,
            grad_year: athlete_grad,
            meet_id: performance.meet.as_str().to_string(),
            meet: meet_name,
            date: performance.date.clone(),
            state: school_state,
            sport: event_sport,
            event: event_name,
            mark: mark_text(&performance.mark),
            normalized,
            timing: performance
                .timing
                .as_ref()
                .map(|t| t.stable_key().to_string()),
            wind_mps: performance.wind_mps,
            round: event_round,
            place: performance.place,
            source,
            source_result: performance.source_key.clone(),
            source_url,
        }
    }
}

fn index<'a, T>(rows: &'a [T], id: impl Fn(&'a T) -> &'a str) -> HashMap<&'a str, &'a T> {
    rows.iter().map(|row| (id(row), row)).collect()
}
