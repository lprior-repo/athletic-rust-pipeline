//! The recruiting read model: one scoped, cohort-filtered pass over the store that the §50, §51 and
//! §53 sheets share.
//!
//! Every value here is read from the store's merged entity tables through [`Store::scan`], filtered
//! by the run's evidence scope exactly as `report` and `bests` filter theirs, so a recruiting cell can
//! never disagree with the census or with the `Best results` sheet. Nothing is re-derived from raw
//! source text: a printed cell traces either to a stored entity field or to a rule over stored fields
//! that the sheet module documents.

use census_domain::JurisdictionBucket;
use crate::report::{in_run_scope, jurisdiction_of, retain_core, school_state_index, ReportResult, Scope};
use crate::store::{Store, Table};
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool,
};
use std::collections::{BTreeMap, HashSet};

use super::contact::{contacts, SchoolContacts};
use super::facts::{
    kind_index, meet_index, pr_index, school_facts, school_index, tally, AthleteTally,
};
use super::prs::{self, PrRow};

/// The measured store audit the three sheets are reconciled against after they are written.
#[derive(Debug, Clone, Copy)]
pub(super) struct Reconciliation {
    /// Merged athlete rows the store returns, before the scope and cohort filters.
    pub(super) store_athletes: usize,
    /// Athlete rows left after the evidence-scope filter.
    pub(super) scoped_athletes: usize,
    /// Athlete rows the `Athletes` sheet publishes.
    pub(super) cohort_athletes: usize,
    /// `(athlete, event)` rows the `PRs` sheet publishes.
    pub(super) pr_rows: usize,
    /// Coach rows the `Coaches` sheet publishes.
    pub(super) coach_rows: usize,
    /// `(slot, side)` head-coach buckets whose rows publish two or more different addresses.
    pub(super) contact_conflicts: usize,
}

/// Everything the recruiting sheets read, loaded once per workbook run.
pub(super) struct Dataset {
    pub(super) scope: Scope,
    pub(super) grad_year: Option<i16>,
    pub(super) athletes: Vec<CanonicalAthlete>,
    /// School id -> school row.
    pub(super) schools: BTreeMap<String, CanonicalSchool>,
    pub(super) coaches: Vec<CanonicalCoach>,
    /// School id -> the contact facts derived from the coach table.
    pub(super) contacts: BTreeMap<String, SchoolContacts>,
    pub(super) tallies: BTreeMap<String, AthleteTally>,
    pub(super) prs: Vec<PrRow>,
    /// Athlete id -> the indices of that athlete's rows in `prs`, in sheet order.
    pub(super) pr_index: BTreeMap<String, Vec<usize>>,
    audit: Reconciliation,
}

impl Dataset {
    /// Load the store into the recruiting read model.
    ///
    /// The scope filter is [`retain_core`], applied to athletes, meets, events and performances in the
    /// same order `bests` applies it, so the core scope of this workbook is the core scope the
    /// platform's best-mark reduction publishes.
    ///
    /// The run scope is also applied: athletes are placed by their school's jurisdiction
    /// (the same rule the census report uses), and coaches are kept only for in-scope schools.
    pub(super) fn load(store: &Store, scope: Scope, grad_year: Option<i16>) -> ReportResult<Self> {
        // Run-scope filter: schools first, so we can place athletes by their school's jurisdiction.
        let mut schools_raw: Vec<CanonicalSchool> = store.scan(Table::Schools)?;
        schools_raw.retain(|s| in_run_scope(JurisdictionBucket::from(s.state)));
        let school_state = school_state_index(&schools_raw);
        let mut athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
        athletes.retain(|a| in_run_scope(jurisdiction_of(&school_state, a.school.as_str())));
        let mut meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
        meets.retain(|m| in_run_scope(JurisdictionBucket::from(m.state)));
        let mut events: Vec<CanonicalEvent> = store.scan(Table::Events)?;
        let mut performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
        if scope == Scope::Core {
            retain_core(&mut athletes);
            retain_core(&mut meets);
            retain_core(&mut events);
            retain_core(&mut performances);
        }
        let store_athletes = athletes.len();
        let scoped_athletes = athletes.len();
        if let Some(year) = grad_year {
            athletes.retain(|athlete| athlete.grad_year.get() == year);
        }
        let schools = school_index(&schools_raw);
        let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
        let scoped_school_ids: HashSet<&str> = schools_raw.iter().map(|s| s.id.as_str()).collect();
        let coaches: Vec<CanonicalCoach> = coaches
            .into_iter()
            .filter(|c| scoped_school_ids.contains(c.school.as_str()))
            .collect();
        let contacts = contacts(&coaches);
        let contact_conflicts: usize = contacts.values().map(|facts| facts.heads.conflicts()).sum();
        let kinds = kind_index(&events);
        let meet_of = meet_index(meets);
        let tallies = tally(&athletes, &performances, &kinds);
        let prs = prs::reduce(
            &athletes,
            &performances,
            &kinds,
            &meet_of,
            &school_facts(&schools),
        );
        let pr_index = pr_index(&prs);
        let audit = Reconciliation {
            store_athletes,
            scoped_athletes,
            cohort_athletes: athletes.len(),
            pr_rows: prs.len(),
            coach_rows: coaches.len(),
            contact_conflicts,
        };
        Ok(Self {
            scope,
            grad_year,
            athletes,
            schools,
            coaches,
            contacts,
            tallies,
            prs,
            pr_index,
            audit,
        })
    }

    pub(super) fn audit(&self) -> Reconciliation {
        self.audit
    }

    /// The school's display name, or its canonical id when the school table has no row for it.
    pub(super) fn school_name(&self, school: &str) -> String {
        self.schools
            .get(school)
            .map(|school| school.name.clone())
            .unwrap_or_else(|| school.to_string())
    }

    /// The school's jurisdiction code, or `UNKNOWN` — the label the census report uses for a school
    /// no row places.
    pub(super) fn school_state(&self, school: &str) -> String {
        self.schools
            .get(school)
            .and_then(|school| school.state)
            .map_or("UNKNOWN".to_string(), |state| state.code().to_string())
    }

    /// The school's city, or empty string when the school table carries none.
    pub(super) fn school_city(&self, school: &str) -> String {
        self.schools
            .get(school)
            .and_then(|s| s.city.clone())
            .unwrap_or_default()
    }

    /// The school's athletics website, when the school table carries one.
    pub(super) fn athletics_url(&self, school: &str) -> String {
        self.schools
            .get(school)
            .and_then(|school| school.athletics_website.clone())
            .unwrap_or_default()
    }

    /// This athlete's PR rows, in sheet order.
    pub(super) fn prs_of(&self, athlete: &str) -> impl Iterator<Item = &PrRow> {
        self.pr_index
            .get(athlete)
            .into_iter()
            .flatten()
            .filter_map(|index| self.prs.get(*index))
    }
}
