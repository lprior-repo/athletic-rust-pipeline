use super::*;
use crate::keys::observation_key;
use crate::read::{sweep_stale_temporaries, write_snapshot_rows};
use census_domain::model::*;
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn school(name: &str) -> CanonicalSchool {
    CanonicalSchool::new(UsJurisdiction::Wisconsin, name, normalize_name(name), None).0
}

fn sequence_pointer(store: &Store, table: Table) -> TestResult<u64> {
    Ok(store.sequences.next_sequence(table)?)
}

fn rows_held(store: &Store, table: Table) -> TestResult<u64> {
    Ok(store.count(table)?)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct DerivedRow {
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

fn derived(id: &str, note: u32) -> DerivedRow {
    DerivedRow {
        id: id.to_string(),
        note,
    }
}

mod batch_atomicity;
mod derived_rows;
mod journal_limits;
mod observation_laws;
mod sequence_bounds;
mod snapshot_publication;
