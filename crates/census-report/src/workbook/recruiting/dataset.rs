use crate::report::{
    exclude_out_of_scope, in_run_scope, jurisdiction_of, retain_core, school_state_index,
    ReportResult, Scope,
};
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalPerformance, CanonicalSchool,
};
use census_store::{Store, Table};
use std::collections::{BTreeMap, HashSet};

use super::contact::{contacts, SchoolContacts};
use super::facts::{kind_index, pr_index, school_index, tally, AthleteTally};
use crate::bests::SharedSelection;

#[derive(Debug, Clone, Copy)]
pub(super) struct Reconciliation {
    pub(super) store_athletes: usize,
    pub(super) scoped_athletes: usize,
    pub(super) cohort_athletes: usize,
    pub(super) pr_rows: usize,
    pub(super) coach_rows: usize,
    pub(super) contact_conflicts: usize,
}

pub(super) struct Dataset {
    pub(super) scope: Scope,
    pub(super) grad_year: Option<i16>,
    pub(super) school_year: census_domain::model::SchoolYear,
    pub(super) athletes: Vec<CanonicalAthlete>,
    pub(super) identities: census_domain::model::AthleteIdentityProjection,
    pub(super) schools: BTreeMap<String, CanonicalSchool>,
    pub(super) coaches: Vec<CanonicalCoach>,
    pub(super) contacts: BTreeMap<String, SchoolContacts>,
    pub(super) tallies: BTreeMap<String, AthleteTally>,
    pub(super) prs: Vec<SharedSelection>,
    pub(super) pr_index: BTreeMap<String, Vec<usize>>,
    audit: Reconciliation,
}

impl Dataset {
    pub(super) fn load(
        store: &Store,
        scope: Scope,
        grad_year: Option<i16>,
        school_year: census_domain::model::SchoolYear,
        prs: Vec<SharedSelection>,
    ) -> ReportResult<Self> {
        Ok(ScopedTables::read(store, scope, grad_year)?.assemble(prs, school_year))
    }

    pub(super) fn audit(&self) -> Reconciliation {
        self.audit
    }

    pub(super) fn school_name(&self, school: &str) -> String {
        self.schools
            .get(school)
            .map(|school| school.name.clone())
            .unwrap_or_else(|| school.to_string())
    }

    pub(super) fn school_state(&self, school: &str) -> String {
        self.schools
            .get(school)
            .and_then(|school| school.state)
            .map_or("UNKNOWN".to_string(), |state| state.code().to_string())
    }

    pub(super) fn school_city(&self, school: &str) -> String {
        self.schools
            .get(school)
            .and_then(|s| s.city.clone())
            .unwrap_or_default()
    }

    pub(super) fn prs_of(&self, athlete: &str) -> impl Iterator<Item = &SharedSelection> {
        self.pr_index
            .get(athlete)
            .into_iter()
            .flatten()
            .filter_map(|index| self.prs.get(*index))
    }
}

struct ScopedTables {
    scope: Scope,
    grad_year: Option<i16>,
    schools: Vec<CanonicalSchool>,
    athletes: Vec<CanonicalAthlete>,
    identities: census_domain::model::AthleteIdentityProjection,
    coaches: Vec<CanonicalCoach>,
    events: Vec<CanonicalEvent>,
    performances: Vec<CanonicalPerformance>,
    store_athletes: usize,
    scoped_athletes: usize,
}

impl ScopedTables {
    fn read(store: &Store, scope: Scope, grad_year: Option<i16>) -> ReportResult<Self> {
        let snapshot = store.snapshot();
        let mut schools: Vec<CanonicalSchool> = snapshot.scan(Table::Schools)?;
        let outside_schools = exclude_out_of_scope(&mut schools, |school| school.state.into());
        let mut school_state = school_state_index(&schools);
        school_state.extend(school_state_index(&outside_schools));
        let mut athletes: Vec<CanonicalAthlete> = snapshot.scan(Table::Athletes)?;
        let store_athletes = athletes.len();
        let mut index = census_domain::model::AthleteIdentityIndex::default();
        for athlete in &athletes {
            index
                .observe(athlete)
                .map_err(census_store::StoreError::from)?;
        }
        let identities = snapshot.project_athlete_identities(index)?;
        athletes.retain(|a| in_run_scope(jurisdiction_of(&school_state, a.school.as_str())));
        let mut events: Vec<CanonicalEvent> = snapshot.scan(Table::Events)?;
        let mut performances: Vec<CanonicalPerformance> = snapshot.scan(Table::Performances)?;
        if scope == Scope::Core {
            retain_core(&mut athletes);
            retain_core(&mut events);
            retain_core(&mut performances);
        }
        let scoped_athletes = athletes.len();
        if let Some(year) = grad_year {
            athletes.retain(|athlete| athlete.grad_year.get() == year);
        }
        let coaches = super::contact::coach_observations(&snapshot)?;
        let scoped_school_ids: HashSet<&str> = schools.iter().map(|s| s.id.as_str()).collect();
        let coaches: Vec<CanonicalCoach> = coaches
            .into_iter()
            .filter(|c| scoped_school_ids.contains(c.school.as_str()))
            .collect();
        Ok(Self {
            scope,
            grad_year,
            schools,
            athletes,
            identities,
            coaches,
            events,
            performances,
            store_athletes,
            scoped_athletes,
        })
    }

    fn assemble(
        self,
        prs: Vec<SharedSelection>,
        school_year: census_domain::model::SchoolYear,
    ) -> Dataset {
        let Self {
            scope,
            grad_year,
            schools: schools_raw,
            athletes,
            identities,
            coaches,
            events,
            performances,
            store_athletes,
            scoped_athletes,
        } = self;
        let schools = school_index(&schools_raw);
        let contacts = contacts(&coaches, school_year);
        let contact_conflicts: usize = contacts.values().map(|facts| facts.heads.conflicts()).sum();
        let kinds = kind_index(&events);
        let tallies = tally(&athletes, &performances, &kinds);
        let pr_index = pr_index(&prs);
        let audit = Reconciliation {
            store_athletes,
            scoped_athletes,
            cohort_athletes: athletes.len(),
            pr_rows: prs.len(),
            coach_rows: coaches.len(),
            contact_conflicts,
        };
        Dataset {
            scope,
            grad_year,
            school_year,
            athletes,
            identities,
            schools,
            coaches,
            contacts,
            tallies,
            prs,
            pr_index,
            audit,
        }
    }
}
