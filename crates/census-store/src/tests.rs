use super::*;
use crate::keys::observation_key;
use crate::read::{sweep_stale_temporaries, write_snapshot_rows};
use census_domain::model::*;
use census_domain::UsJurisdiction;

pub(crate) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub(crate) fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name), None).0
}

fn sequence_pointer(store: &Store, table: Table) -> TestResult<u64> {
    Ok(store.sequences.next_sequence(table)?)
}

fn rows_held(store: &Store, table: Table) -> TestResult<u64> {
    Ok(store.count(table)?)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct DerivedRow {
    id: String,
    note: u32,
}

impl Entity for DerivedRow {
    fn entity_id(&self) -> &str {
        &self.id
    }

    fn merge(&mut self, other: Self) {
        *self = other;
    }
}

pub(crate) fn derived(id: &str, note: u32) -> DerivedRow {
    DerivedRow {
        id: id.to_string(),
        note,
    }
}

pub(crate) fn generation_rows(store: &Store, table: Table, generation: u64) -> TestResult<u64> {
    let prefix = crate::keys::derived_generation_prefix(table, generation);
    let mut held = 0_u64;
    for guard in store.entities.prefix(prefix.as_slice()) {
        guard.key().map_err(|source| StoreError::Read { source })?;
        held = held.checked_add(1).ok_or(StoreError::CounterOverflow)?;
    }
    Ok(held)
}

mod batch_atomicity;
mod derived_publication;
mod derived_rows;
mod journal_limits;
mod observation_laws;
mod sequence_bounds;
mod snapshot_publication;
