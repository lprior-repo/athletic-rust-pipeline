use fjall::{Keyspace, OwnedWriteBatch, Snapshot};

use crate::{Store, StoreError, StoreResult, Table};

mod codec;
mod enrollment;
mod format;
#[cfg(test)]
mod tests;

pub(crate) use enrollment::enroll_locked;
pub(crate) use format::{preflight_keyspaces, validate_format};

const KEY: &str = "capability:public-identity-input-revisions-v2";
const PREFIX: &str = "capability:public-identity-input-revisions-";
const TABLES: [Table; 6] = [
    Table::Athletes,
    Table::SourceObservations,
    Table::ReviewCases,
    Table::IdentityVerdicts,
    Table::Performances,
    Table::RelayResults,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PublicIdentityInputRevisionsV2 {
    Disabled,
    Enabled(PublicIdentityInputRevisionTokenV2),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicIdentityInputRevisionTokenV2 {
    baseline_evidence_generation: u64,
    baseline_derived_generation: u64,
    counters: [u64; 6],
}

impl PublicIdentityInputRevisionTokenV2 {
    pub fn table_revision(&self, table: Table) -> StoreResult<u64> {
        TABLES
            .iter()
            .position(|dependency| *dependency == table)
            .and_then(|index| self.counters.get(index))
            .copied()
            .ok_or_else(|| StoreError::Refused {
                detail: format!("{} is not a public identity input dependency", table.file()),
            })
    }

    fn advance(mut self, touched: [bool; 6]) -> StoreResult<Self> {
        for (counter, touched) in self.counters.iter_mut().zip(touched) {
            if touched {
                *counter = counter.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            }
        }
        Ok(self)
    }
}

pub(crate) fn read(
    snapshot: &Snapshot,
    meta: &Keyspace,
) -> StoreResult<PublicIdentityInputRevisionsV2> {
    let state = codec::read(snapshot, meta)?;
    if let PublicIdentityInputRevisionsV2::Enabled(token) = state {
        format::validate_baseline(snapshot, meta, token)?;
    }
    Ok(state)
}

pub(crate) fn stage(
    store: &Store,
    batch: &mut OwnedWriteBatch,
    tables: impl IntoIterator<Item = Table>,
) -> StoreResult<()> {
    let mut touched = [false; 6];
    for table in tables {
        for (dependency, touched) in TABLES.into_iter().zip(touched.iter_mut()) {
            *touched |= dependency == table;
        }
    }
    if !touched.contains(&true) {
        return Ok(());
    }
    match read(&store.db.snapshot(), &store.meta)? {
        PublicIdentityInputRevisionsV2::Disabled => Ok(()),
        PublicIdentityInputRevisionsV2::Enabled(token) => {
            codec::write(batch, &store.meta, token.advance(touched)?)
        }
    }
}

fn invalid(detail: &str) -> StoreError {
    StoreError::SchemaUnknown {
        detail: format!("{KEY}: {detail}"),
    }
}
