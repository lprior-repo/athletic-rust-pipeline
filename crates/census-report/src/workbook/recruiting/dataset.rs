use crate::bests::SharedSelection;
use crate::report::{Derivation, ReportResult, Scope};
use census_domain::model::{CanonicalAthlete, CanonicalCoach, CanonicalSchool, SchoolYear};
use std::collections::BTreeMap;

use super::contact::{contacts, SchoolContacts};
use super::facts::{kind_index, pr_index, school_index, tally, AthleteTally};

use super::coalesce;

#[derive(Debug, Clone, Copy)]
pub(super) struct Reconciliation {
    pub(super) store_athletes: usize,
    pub(super) scoped_athletes: usize,
    pub(super) cohort_athletes: usize,
    pub(super) pr_rows: usize,
    pub(super) coach_rows: usize,
    pub(super) published_coach_rows: usize,
    pub(super) contact_conflicts: usize,
}

pub(super) struct Dataset {
    pub(super) scope: Scope,
    pub(super) grad_year: Option<i16>,
    pub(super) school_year: census_domain::model::SchoolYear,
    pub(super) athletes: Vec<CanonicalAthlete>,
    pub(super) identities: std::sync::Arc<census_domain::model::AthleteIdentityProjection>,
    pub(super) schools: BTreeMap<census_domain::model::SchoolId, CanonicalSchool>,
    pub(super) published_coaches: Vec<CanonicalCoach>,
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
        let inputs = Inputs::new(derivation, school_year, prs);
        let contacts = contacts(derivation.coach_observations(), school_year);
        let audit = inputs.audit(&contacts);
        Ok(Self::assemble(
            inputs,
            athletes,
            school_address,
            contacts,
            audit,
        ))
    }

    fn assemble(
        inputs: Inputs<'_, '_>,
        athletes: Vec<CanonicalAthlete>,
        school_address: BTreeMap<String, String>,
        contacts: BTreeMap<String, SchoolContacts>,
        audit: Reconciliation,
    ) -> Self {
        Self {
            scope: inputs.derivation.scope(),
            grad_year: inputs.derivation.grad_year(),
            school_year: inputs.school_year,
            identities: inputs.derivation.dataset().identities(),
            schools: school_index(inputs.derivation.schools()),
            published_coaches: coalesce::claims(inputs.derivation.coach_observations()),
            tallies: tallies_of(inputs.derivation, &athletes),
            athletes,
            contacts,
            pr_index: pr_index(&inputs.prs),
            prs: inputs.prs,
            school_address,
            audit,
        }
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

struct Inputs<'a, 'd> {
    derivation: &'a Derivation<'d>,
    school_year: SchoolYear,
    prs: Vec<SharedSelection>,
}

impl<'a, 'd> Inputs<'a, 'd> {
    fn new(
        derivation: &'a Derivation<'d>,
        school_year: SchoolYear,
        prs: Vec<SharedSelection>,
    ) -> Self {
        Self {
            derivation,
            school_year,
            prs,
        }
    }

    fn audit(&self, contacts: &BTreeMap<String, SchoolContacts>) -> Reconciliation {
        Reconciliation {
            store_athletes: self.derivation.dataset().athletes.len(),
            scoped_athletes: self.derivation.scoped_athletes(),
            cohort_athletes: self.derivation.athletes().len(),
            pr_rows: self.prs.len(),
            coach_rows: self.derivation.coach_observations().len(),
            published_coach_rows: coalesce::claims(self.derivation.coach_observations()).len(),
            contact_conflicts: contacts.values().map(|facts| facts.heads.conflicts()).sum(),
        }
    }
}

fn tallies_of(
    derivation: &Derivation<'_>,
    athletes: &[CanonicalAthlete],
) -> BTreeMap<String, AthleteTally> {
    let kinds = kind_index(derivation.events());
    tally(
        athletes,
        derivation.performances(),
        &kinds,
        derivation.athlete_aliases(),
    )
}
