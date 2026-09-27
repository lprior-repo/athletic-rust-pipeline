use crate::report::{in_run_scope, retain_core_row, Scope};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, EventKind,
};
use census_store::{StoreResult, Table};
use std::collections::{BTreeMap, HashSet};

pub(crate) struct Referenced {
    athletes: HashSet<String>,
}

impl Referenced {
    pub(crate) fn collect(snapshot: &census_store::StoreSnapshot<'_>) -> StoreResult<Self> {
        let mut athletes = HashSet::new();
        snapshot.for_each_merged(Table::Performances, |performance: CanonicalPerformance| {
            athletes.insert(performance.athlete.as_str().to_string());
            Ok(())
        })?;
        Ok(Self { athletes })
    }
}

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
        referenced: &Referenced,
    ) -> StoreResult<Self> {
        let schools = load_schools(snapshot)?;
        let athletes = load_athletes(snapshot, &schools, scope, grad_year, &referenced.athletes)?;
        let meets = load_meets(snapshot, scope)?;
        let events = load_events(snapshot, scope)?;
        let teams = load_teams(snapshot)?;
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

fn load_schools(
    snapshot: &census_store::StoreSnapshot<'_>,
) -> StoreResult<BTreeMap<String, CanonicalSchool>> {
    let mut schools: BTreeMap<String, CanonicalSchool> = BTreeMap::new();
    snapshot.for_each_merged(Table::Schools, |school: CanonicalSchool| {
        schools.insert(school.id.as_str().to_string(), school);
        Ok(())
    })?;
    Ok(schools)
}

fn load_athletes(
    snapshot: &census_store::StoreSnapshot<'_>,
    schools: &BTreeMap<String, CanonicalSchool>,
    scope: Scope,
    grad_year: Option<i16>,
    selected: &HashSet<String>,
) -> StoreResult<BTreeMap<String, CanonicalAthlete>> {
    let mut athletes: BTreeMap<String, CanonicalAthlete> = BTreeMap::new();
    snapshot.for_each_merged_selected(
        Table::Athletes,
        selected,
        |mut athlete: CanonicalAthlete| {
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
        },
    )?;
    Ok(athletes)
}

fn load_meets(
    snapshot: &census_store::StoreSnapshot<'_>,
    scope: Scope,
) -> StoreResult<BTreeMap<String, CanonicalMeet>> {
    let mut meets: BTreeMap<String, CanonicalMeet> = BTreeMap::new();
    snapshot.for_each_merged(Table::Meets, |mut meet: CanonicalMeet| {
        if scope == Scope::Core && !retain_core_row(&mut meet) {
            return Ok(());
        }
        meets.insert(meet.id.as_str().to_string(), meet);
        Ok(())
    })?;
    Ok(meets)
}

fn load_events(
    snapshot: &census_store::StoreSnapshot<'_>,
    scope: Scope,
) -> StoreResult<BTreeMap<String, EventKind>> {
    let mut events: BTreeMap<String, EventKind> = BTreeMap::new();
    snapshot.for_each_merged(Table::Events, |mut event: CanonicalEvent| {
        if scope == Scope::Core && !retain_core_row(&mut event) {
            return Ok(());
        }
        events.insert(event.id.as_str().to_string(), event.kind);
        Ok(())
    })?;
    Ok(events)
}

fn load_teams(
    snapshot: &census_store::StoreSnapshot<'_>,
) -> StoreResult<BTreeMap<String, CanonicalTeam>> {
    let mut teams: BTreeMap<String, CanonicalTeam> = BTreeMap::new();
    snapshot.for_each_merged(Table::Teams, |team: CanonicalTeam| {
        teams.insert(team.id.as_str().to_string(), team);
        Ok(())
    })?;
    Ok(teams)
}
