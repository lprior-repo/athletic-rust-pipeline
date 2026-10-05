use crate::bests::SharedSelection;
use crate::report::{Derivation, ReportResult, Scope};
use census_domain::model::{CanonicalAthlete, CanonicalCoach, CanonicalSchool};
use std::collections::BTreeMap;

use super::contact::{contacts, SchoolContacts};
use super::facts::{kind_index, pr_index, school_index, tally, AthleteTally};

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
    pub(super) identities: std::sync::Arc<census_domain::model::AthleteIdentityProjection>,
    pub(super) schools: BTreeMap<census_domain::model::SchoolId, CanonicalSchool>,
    pub(super) coaches: Vec<CanonicalCoach>,
    pub(super) contacts: BTreeMap<String, SchoolContacts>,
    pub(super) tallies: BTreeMap<String, AthleteTally>,
    pub(super) prs: Vec<SharedSelection>,
    pub(super) school_address: BTreeMap<String, String>,
    pub(super) pr_index: BTreeMap<String, Vec<usize>>,
    audit: Reconciliation,
}

impl Dataset {
    pub(super) fn of(
        derivation: &Derivation<'_>,
        school_year: census_domain::model::SchoolYear,
        prs: Vec<SharedSelection>,
    ) -> ReportResult<Self> {
        let athletes = derivation.athletes().to_vec();
        let school_address =
            crate::export::postal::athlete_address_index(derivation.dataset(), &athletes)?;
        let identities = derivation.dataset().identities();
        let schools = school_index(derivation.schools());
        let coaches = derivation.coach_observations().to_vec();
        let events = derivation.events().to_vec();
        let performances = derivation.performances().to_vec();
        let contacts = contacts(&coaches, school_year);
        let contact_conflicts: usize = contacts.values().map(|facts| facts.heads.conflicts()).sum();
        let kinds = kind_index(&events);
        let tallies = tally(
            &athletes,
            &performances,
            &kinds,
            derivation.athlete_aliases(),
        );
        let pr_index = pr_index(&prs);
        let audit = Reconciliation {
            store_athletes: derivation.dataset().athletes.len(),
            scoped_athletes: derivation.scoped_athletes(),
            cohort_athletes: athletes.len(),
            pr_rows: prs.len(),
            coach_rows: coaches.len(),
            contact_conflicts,
        };
        Ok(Dataset {
            scope: derivation.scope(),
            grad_year: derivation.grad_year(),
            school_year,
            athletes,
            identities,
            schools,
            coaches,
            contacts,
            tallies,
            prs,
            school_address,
            pr_index,
            audit,
        })
    }

    pub(super) fn audit(&self) -> Reconciliation {
        self.audit
    }

    pub(super) fn school_name(&self, school: &str) -> String {
        self.schools
            .get(school)
            .map(|s| s.name.clone())
            .map_or(Default::default(), core::convert::identity)
    }

    pub(super) fn school_state(&self, school: &str) -> String {
        self.schools
            .get(school)
            .and_then(|s| s.state.map(|st| st.to_string()))
            .map_or(Default::default(), core::convert::identity)
    }

    pub(super) fn school_city(&self, school: &str) -> String {
        self.schools
            .get(school)
            .and_then(|s| s.city.clone())
            .map_or(Default::default(), core::convert::identity)
    }

    pub(super) fn prs_of(&self, athlete: &str) -> Box<dyn Iterator<Item = &SharedSelection> + '_> {
        match self.pr_index.get(athlete) {
            Some(idxs) => Box::new(idxs.iter().filter_map(|&i| self.prs.get(i))),
            None => Box::new(std::iter::empty()),
        }
    }
}
