use std::collections::BTreeSet;

use census_domain::model::{CanonicalSchool, ReviewCase, SchoolId};
use census_domain::school_directory::{DirectoryIndex, MAX_CO_OP_MEMBERS};
use census_store::{Store, StoreError, StoreResult, StoreRevision, StoreSnapshot, Table};

use super::super::lanes::ASSOCIATION_PREFIX;
use super::super::receipts::{apply_target, ApplicationScope};
use super::super::support::allocation;
use super::super::{Counters, Job, JoinError, LaneSet, Mode, OutcomeRow};

const OUTCOME_RESERVE: usize = 1024;

pub(in crate::school_address::join) enum ApplicationContext<'s> {
    DryRun,
    Apply {
        scope: ApplicationScope,
        revision: StoreRevision<'s>,
    },
}

impl<'s> ApplicationContext<'s> {
    fn new(lanes: &LaneSet, mode: Mode, snapshot: &StoreSnapshot<'s>) -> StoreResult<Self> {
        match mode {
            Mode::DryRun => Ok(Self::DryRun),
            Mode::Apply => Ok(Self::Apply {
                scope: ApplicationScope::new(lanes)?,
                revision: snapshot.revision(),
            }),
        }
    }

    fn apply_target(
        &mut self,
        store: &'s Store,
        target: &SchoolId,
        schools: &[CanonicalSchool],
        cases: &[ReviewCase],
    ) -> StoreResult<()> {
        let Self::Apply { scope, revision } = self else {
            return Ok(());
        };
        *revision = apply_target(store, scope, target, schools, cases, *revision)?;
        Ok(())
    }
}

impl<'a> Job<'a> {
    pub(in crate::school_address::join) fn new(
        index: &'a DirectoryIndex,
        lanes: &'a LaneSet,
        mode: Mode,
        store: &'a Store,
        snapshot: &StoreSnapshot<'a>,
    ) -> Result<Self, JoinError> {
        let application = ApplicationContext::new(lanes, mode, snapshot)?;
        let mut outcomes: Vec<OutcomeRow> = Vec::new();
        outcomes
            .try_reserve(OUTCOME_RESERVE)
            .map_err(|error| allocation(error.to_string()))?;
        let mut pending: Vec<CanonicalSchool> = Vec::new();
        let capacity = MAX_CO_OP_MEMBERS
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        pending
            .try_reserve_exact(capacity)
            .map_err(|error| allocation(error.to_string()))?;
        let mut pending_cases = Vec::new();
        pending_cases
            .try_reserve_exact(capacity)
            .map_err(|error| allocation(error.to_string()))?;
        let existing_cases: BTreeSet<String> = if mode == Mode::Apply {
            snapshot
                .scan::<ReviewCase>(Table::ReviewCases)?
                .into_iter()
                .map(|case| case.id)
                .collect()
        } else {
            BTreeSet::new()
        };
        let (association, association_problem) = association_lanes(lanes);
        Ok(Self {
            index,
            lanes,
            mode,
            store,
            counters: Counters::default(),
            outcomes,
            pending,
            existing_cases,
            pending_cases,
            application,
            association,
            association_problem,
            attested: Vec::new(),
            postal_present: false,
            postal_provider: None,
        })
    }

    pub(in crate::school_address::join) fn finish(
        self,
    ) -> StoreResult<(Counters, Vec<OutcomeRow>)> {
        if !self.pending.is_empty() || !self.pending_cases.is_empty() {
            return Err(StoreError::Invariant {
                detail: "school-address join finished with an unapplied target group".into(),
            });
        }
        Ok((self.counters, self.outcomes))
    }

    pub(in crate::school_address::join) fn apply_target(
        &mut self,
        target: &SchoolId,
    ) -> StoreResult<()> {
        self.application
            .apply_target(self.store, target, &self.pending, &self.pending_cases)?;
        self.pending.clear();
        self.pending_cases.clear();
        Ok(())
    }
}

fn association_lanes(lanes: &LaneSet) -> (Option<String>, Option<String>) {
    let mut association: Option<String> = None;
    let mut problem: Option<String> = None;
    for key in lanes.tokens() {
        let Some(slug) = key.strip_prefix(ASSOCIATION_PREFIX) else {
            continue;
        };
        match &association {
            None => association = Some(slug.to_string()),
            Some(first) if first == slug => {}
            Some(first) => {
                problem = Some(format!(
                    "the generation admits several association lanes ({first}, {slug}); \
                     build one association lane per generation"
                ));
            }
        }
    }
    (association, problem)
}
