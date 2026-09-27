use crate::report::{in_run_scope, retain_core_row, Scope};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalSchool, CanonicalTeam, EventKind,
};
use census_store::{StoreResult, Table};
use std::collections::BTreeMap;

pub(crate) struct Parents {
    pub(crate) athletes: BTreeMap<String, CanonicalAthlete>,
    pub(crate) meets: BTreeMap<String, CanonicalMeet>,
    pub(crate) events: BTreeMap<String, EventKind>,
    pub(crate) teams: BTreeMap<String, CanonicalTeam>,
    pub(crate) schools: BTreeMap<String, CanonicalSchool>,
}

impl Parents {
    pub(crate) fn read(
        snapshot: &census_store::StoreSnapshot<'_>,
        scope: Scope,
        grad_year: Option<i16>,
    ) -> StoreResult<Self> {
        let mut athletes: BTreeMap<String, CanonicalAthlete> = BTreeMap::new();
        let mut meets: BTreeMap<String, CanonicalMeet> = BTreeMap::new();
        let mut events: BTreeMap<String, EventKind> = BTreeMap::new();
        let mut teams: BTreeMap<String, CanonicalTeam> = BTreeMap::new();
        let mut schools: BTreeMap<String, CanonicalSchool> = BTreeMap::new();
        snapshot.for_each_merged(Table::Schools, |s: CanonicalSchool| {
            let id = s.id.as_str().to_string();
            schools.insert(id, s);
            Ok(())
        })?;

        snapshot.for_each_merged(Table::Athletes, |mut athlete: CanonicalAthlete| {
            if scope == Scope::Core && !retain_core_row(&mut athlete) {
                return Ok(());
            }
            if grad_year.is_some_and(|year| athlete.grad_year.get() != year) {
                return Ok(());
            }
            let state = schools
                .get(athlete.school.as_str())
                .and_then(|school| school.state);
            if in_run_scope(state.into()) {
                athletes.insert(athlete.id.as_str().to_string(), athlete);
            }
            Ok(())
        })?;

        snapshot.for_each_merged(Table::Meets, |mut m: CanonicalMeet| {
            if scope == Scope::Core && !retain_core_row(&mut m) {
                return Ok(());
            }
            meets.insert(m.id.as_str().to_string(), m);
            Ok(())
        })?;

        snapshot.for_each_merged(Table::Events, |mut e: CanonicalEvent| {
            if scope == Scope::Core && !retain_core_row(&mut e) {
                return Ok(());
            }
            let id = e.id.as_str().to_string();
            events.insert(id, e.kind);
            Ok(())
        })?;

        snapshot.for_each_merged(Table::Teams, |t: CanonicalTeam| {
            let id = t.id.as_str().to_string();
            teams.insert(id, t);
            Ok(())
        })?;

        Ok(Self {
            athletes,
            meets,
            events,
            teams,
            schools,
        })
    }

    pub(crate) fn athlete(&self, id: &str) -> Option<&CanonicalAthlete> {
        self.athletes.get(id)
    }

    pub(crate) fn meet(&self, id: &str) -> Option<&CanonicalMeet> {
        self.meets.get(id)
    }

    pub(crate) fn event(&self, id: &str) -> Option<&EventKind> {
        self.events.get(id)
    }

    pub(crate) fn team(&self, id: &str) -> Option<&CanonicalTeam> {
        self.teams.get(id)
    }

    pub(crate) fn school(&self, id: &str) -> Option<&CanonicalSchool> {
        self.schools.get(id)
    }
}
