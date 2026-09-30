use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam,
};
use census_store::clock::{Clock, SystemClock};
use census_store::{Store, Table};
use std::collections::BTreeMap;

use crate::report::Scope;
use crate::report::{
    exclude_out_of_scope, in_run_scope, jurisdiction_of, retain_core, ReportResult,
};

#[derive(Debug)]
pub struct ExportDataset {
    pub athletes: Vec<CanonicalAthlete>,
    pub schools: BTreeMap<String, CanonicalSchool>,
    pub coaches: Vec<CanonicalCoach>,
    pub events: Vec<CanonicalEvent>,
    pub performances: Vec<CanonicalPerformance>,
    pub meets: Vec<CanonicalMeet>,
    pub teams: BTreeMap<String, CanonicalTeam>,
    pub generated_on: String,
}

impl ExportDataset {
    pub fn load(store: &Store, scope: Scope, grad_year: Option<i16>) -> ReportResult<Self> {
        let snapshot = store.snapshot();
        let generated_on = SystemClock.today();

        let mut athletes: Vec<CanonicalAthlete> = snapshot.scan(Table::Athletes)?;
        let mut meets: Vec<CanonicalMeet> = snapshot.scan(Table::Meets)?;
        let mut events: Vec<CanonicalEvent> = snapshot.scan(Table::Events)?;
        let mut performances: Vec<CanonicalPerformance> = snapshot.scan(Table::Performances)?;

        if scope == Scope::Core {
            retain_core(&mut athletes);
            retain_core(&mut meets);
            retain_core(&mut events);
            retain_core(&mut performances);
        }

        let mut schools: Vec<CanonicalSchool> = snapshot.scan(Table::Schools)?;
        let outside_schools = exclude_out_of_scope(&mut schools, |school| school.state.into());
        let mut school_state = crate::report::school_state_index(&schools);
        school_state.extend(crate::report::school_state_index(&outside_schools));
        athletes.retain(|a| in_run_scope(jurisdiction_of(&school_state, a.school.as_str())));
        meets.retain(|m| in_run_scope(census_domain::JurisdictionBucket::from(m.state)));

        if let Some(year) = grad_year {
            athletes.retain(|a| a.grad_year.get() == year);
        }

        let schools = schools
            .into_iter()
            .map(|s| (s.id.as_str().to_string(), s))
            .collect();

        let teams: BTreeMap<String, CanonicalTeam> = snapshot
            .scan(Table::Teams)?
            .into_iter()
            .map(|t: CanonicalTeam| (t.id.as_str().to_string(), t))
            .collect();

        let coaches = snapshot.scan(Table::Coaches)?;

        Ok(Self {
            athletes,
            schools,
            coaches,
            events,
            performances,
            meets,
            teams,
            generated_on,
        })
    }

    pub fn scope(&self) -> ScopedDataset<'_> {
        ScopedDataset {
            athletes: &self.athletes,
            schools: &self.schools,
            coaches: &self.coaches,
            events: &self.events,
            performances: &self.performances,
            meets: &self.meets,
            teams: &self.teams,
            generated_on: self.generated_on.as_str(),
        }
    }
}

#[derive(Debug)]
pub struct ScopedDataset<'a> {
    pub athletes: &'a [CanonicalAthlete],
    pub schools: &'a BTreeMap<String, CanonicalSchool>,
    pub coaches: &'a [CanonicalCoach],
    pub events: &'a [CanonicalEvent],
    pub performances: &'a [CanonicalPerformance],
    pub meets: &'a [CanonicalMeet],
    pub teams: &'a BTreeMap<String, CanonicalTeam>,
    pub generated_on: &'a str,
}
