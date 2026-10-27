use crate::bests::SharedSelection;
use crate::report::{Derivation, ReportResult, Scope};
use census_domain::model::{CanonicalAthlete, CanonicalCoach, CanonicalSchool, SchoolYear};
use std::collections::BTreeMap;

use super::bounded::{bounded_claims, bounded_contacts, WORKBOOK_GROUP_BUDGET};
use super::contact::{attach_research, SchoolContacts};
use super::facts::{kind_index, pr_index, school_index, tally, AthleteTally};
use crate::workbook::corpus_address::{CorpusAddresses, QuarantineRow};

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

pub(super) struct Dataset<'a> {
    pub(super) scope: Scope,
    pub(super) grad_year: Option<i16>,
    pub(super) school_year: census_domain::model::SchoolYear,
    pub(super) athletes: &'a [CanonicalAthlete],
    pub(super) public_identities:
        Option<&'a crate::export::public_identity::VerifiedPublicIdentityView>,
    pub(super) source: &'a crate::export::ExportDataset,
    pub(super) schools: BTreeMap<&'a str, &'a CanonicalSchool>,
    pub(super) published_coaches: Vec<CanonicalCoach>,
    pub(super) coach_spellings: BTreeMap<String, String>,
    pub(super) contacts: BTreeMap<String, SchoolContacts>,
    pub(super) tallies: BTreeMap<String, AthleteTally>,
    pub(super) prs: Vec<SharedSelection>,
    pub(super) school_address: BTreeMap<String, String>,
    pub(super) pr_index: BTreeMap<String, Vec<usize>>,
    address_quarantine: Vec<QuarantineRow>,
    audit: Reconciliation,
}

impl<'a> Dataset<'a> {
    pub(super) fn of<'d>(
        derivation: &'a Derivation<'d>,
        school_year: census_domain::model::SchoolYear,
        prs: Vec<SharedSelection>,
        admission: &crate::report::EventAdmission<'d>,
        corpus: Option<&CorpusAddresses>,
    ) -> ReportResult<Self> {
        derivation.ensure_selected_affiliation()?;
        crate::workbook::public_identity::preflight(derivation.dataset(), derivation.athletes())?;
        crate::bests::participation::ParticipationSelection::of(derivation.dataset())?;
        admission.ensure_dataset(derivation.dataset())?;
        crate::grouping::Admission::new("recruiting schools").rows(derivation.schools())?;
        let athletes = derivation.athletes();
        let school_address =
            crate::export::postal::athlete_address_index(derivation.dataset(), athletes)?;
        let inputs = Inputs::new(derivation, school_year, prs, admission);
        let mut contacts = bounded_contacts(
            derivation.coach_observations(),
            school_year,
            WORKBOOK_GROUP_BUDGET,
        )?;
        attach_research(&mut contacts, derivation.schools(), school_year);
        let published_coaches =
            bounded_claims(derivation.coach_observations(), WORKBOOK_GROUP_BUDGET)?;
        let audit = inputs.audit(&contacts, published_coaches.len());
        Self::assemble(
            inputs,
            athletes,
            school_address,
            contacts,
            published_coaches,
            audit,
            corpus,
        )
    }

    fn assemble(
        inputs: Inputs<'a, '_, '_>,
        athletes: &'a [CanonicalAthlete],
        mut school_address: BTreeMap<String, String>,
        contacts: BTreeMap<String, SchoolContacts>,
        published_coaches: Vec<CanonicalCoach>,
        audit: Reconciliation,
        corpus: Option<&CorpusAddresses>,
    ) -> ReportResult<Self> {
        let schools = school_index(inputs.derivation.schools());
        let address_quarantine = corpus
            .map(|corpus| corpus.fill_gaps(&mut school_address, athletes, &schools))
            .unwrap_or_default();
        Ok(Self {
            scope: inputs.derivation.scope(),
            grad_year: inputs.derivation.grad_year(),
            school_year: inputs.school_year,
            source: inputs.derivation.dataset(),
            public_identities: crate::workbook::public_identity::view(inputs.derivation.dataset())?,
            schools,
            published_coaches,
            coach_spellings: super::coach_spelling::spellings(
                inputs.derivation.coach_observations(),
            ),
            tallies: tallies_of(inputs.derivation, athletes, inputs.admission),
            athletes,
            contacts,
            pr_index: pr_index(&inputs.prs),
            prs: inputs.prs,
            school_address,
            address_quarantine,
            audit,
        })
    }

    pub(super) fn audit(&self) -> Reconciliation {
        self.audit
    }

    pub(super) fn address_quarantine(&self) -> &[QuarantineRow] {
        &self.address_quarantine
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

struct Inputs<'a, 'd, 'e> {
    derivation: &'a Derivation<'d>,
    school_year: SchoolYear,
    prs: Vec<SharedSelection>,
    admission: &'e crate::report::EventAdmission<'d>,
}

impl<'a, 'd, 'e> Inputs<'a, 'd, 'e> {
    fn new(
        derivation: &'a Derivation<'d>,
        school_year: SchoolYear,
        prs: Vec<SharedSelection>,
        admission: &'e crate::report::EventAdmission<'d>,
    ) -> Self {
        Self {
            derivation,
            school_year,
            prs,
            admission,
        }
    }

    fn audit(
        &self,
        contacts: &BTreeMap<String, SchoolContacts>,
        published_coach_rows: usize,
    ) -> Reconciliation {
        Reconciliation {
            store_athletes: self.derivation.dataset().athletes.len(),
            scoped_athletes: self.derivation.scoped_athletes(),
            cohort_athletes: self.derivation.athletes().len(),
            pr_rows: self.prs.len(),
            coach_rows: self.derivation.coach_observations().len(),
            published_coach_rows,
            contact_conflicts: contacts.values().map(|facts| facts.heads.conflicts()).sum(),
        }
    }
}

fn tallies_of(
    derivation: &Derivation<'_>,
    athletes: &[CanonicalAthlete],
    admission: &crate::report::EventAdmission<'_>,
) -> BTreeMap<String, AthleteTally> {
    let kinds = kind_index(derivation.events(), admission);
    tally(
        athletes,
        derivation.performances(),
        &kinds,
        derivation.athlete_aliases(),
    )
}
