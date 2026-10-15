use super::*;
use crate::public_identity_tests::support::{fixture, retained_state, subject, PhysicalState};
use crate::tests::{derived, stage_required_tables, DerivedRow, TestResult};
use crate::{Application, StorageMode};
use fjall::PersistMode;
use serde_json::json;

mod corruption;
mod enrollment;
mod interleavings;
mod lineage;
mod mutations;
mod overflow;
mod publication;
mod snapshots;

fn enabled(store: &Store) -> TestResult<PublicIdentityInputRevisionTokenV2> {
    enabled_snapshot(&store.snapshot())
}

fn enabled_snapshot(
    snapshot: &crate::StoreSnapshot<'_>,
) -> TestResult<PublicIdentityInputRevisionTokenV2> {
    match snapshot.public_identity_input_revisions_v2()? {
        PublicIdentityInputRevisionsV2::Enabled(token) => Ok(token),
        PublicIdentityInputRevisionsV2::Disabled => Err("expected explicit enrollment".into()),
    }
}

fn enroll(store: &Store) -> TestResult<PublicIdentityInputRevisionTokenV2> {
    Ok(store.enroll_public_identity_input_revisions_v2(store.snapshot().revision())?)
}

fn assert_counters(store: &Store, expected: [u64; 6]) -> TestResult {
    let token = enabled(store)?;
    for (table, expected) in TABLES.into_iter().zip(expected) {
        check!(eq; token.table_revision(table)?, expected);
    }
    Ok(())
}

fn mutate(store: &Store, table: Table, id: &str) -> TestResult {
    let row = derived(id, 1);
    match table.storage_mode() {
        StorageMode::ObservationLog => store.append(table, &row)?,
        StorageMode::DerivedMap | StorageMode::DerivedGeneration => store.replace(table, &row)?,
    }
    Ok(())
}

fn stage_counters(store: &Store, counters: [u64; 6]) -> TestResult {
    let mut token = enabled(store)?;
    token.counters = counters;
    let mut batch = store.db.batch();
    codec::write(&mut batch, &store.meta, token)?;
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    Ok(())
}

fn without_capability(mut state: PhysicalState) -> PhysicalState {
    for (name, rows) in &mut state {
        if name == crate::META {
            rows.retain(|(key, _)| key.as_slice() != KEY.as_bytes());
        }
    }
    state
}
