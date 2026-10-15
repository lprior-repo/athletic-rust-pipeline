mod read;
mod storage;
mod wire;
pub(super) use read::read as read_inputs;
#[cfg(test)]
pub(super) use read::verify_reviewed;
mod verify;
pub(super) use verify::{reviewed as verify_reviewed_inputs, source as verify_source_inputs};
mod decode;
pub(super) use decode::from_bytes;

use crate::{Entity, StoreError, StoreResult, StoreSnapshot, Table};
use census_domain::model::{CanonicalAthlete, ReviewCase, ReviewVerdictRecord, SourceObservation};

pub(super) const BUDGET: crate::read::ReadBudget = crate::read::ReadBudget {
    rows: 8192,
    bytes: 16 * 1024 * 1024,
    record_bytes: 1024 * 1024,
};

pub(super) struct Inputs {
    pub athletes: Vec<CanonicalAthlete>,
    pub observations: Vec<SourceObservation>,
    pub cases: Vec<ReviewCase>,
    pub verdicts: Vec<ReviewVerdictRecord>,
}

impl Inputs {
    pub(super) fn capture(snapshot: &StoreSnapshot<'_>) -> StoreResult<Self> {
        let athletes = collect::<CanonicalAthlete>(snapshot, Table::Athletes)?;
        let mut observations = Vec::new();
        snapshot.for_each_observation_bounded(Table::SourceObservations, BUDGET, |row, _| {
            push(&mut observations, row)
        })?;
        Ok(Self {
            athletes,
            observations,
            cases: collect(snapshot, Table::ReviewCases)?,
            verdicts: collect(snapshot, Table::IdentityVerdicts)?,
        })
    }
}

fn collect<T: Entity>(snapshot: &StoreSnapshot<'_>, table: Table) -> StoreResult<Vec<T>> {
    let mut rows = Vec::new();
    snapshot.for_each_merged_bounded(table, BUDGET, |row, _| push(&mut rows, row))?;
    Ok(rows)
}

fn push<T>(rows: &mut Vec<T>, row: T) -> StoreResult<()> {
    rows.try_reserve(1).map_err(|error| StoreError::Refused {
        detail: format!("public identity input allocation: {error}"),
    })?;
    rows.push(row);
    Ok(())
}

#[cfg(test)]
mod test_fixture;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod historical_witness_tests;

#[cfg(test)]
mod consumer_snapshot_tests;

#[cfg(test)]
mod section_budget_tests;

#[cfg(test)]
pub(in crate::public_identity) mod reviewed_fixture;

#[cfg(test)]
mod reviewed_consumer_tests;

#[cfg(test)]
mod local_adjudication_tests;
