use super::*;
use crate::keys::{observation_key, DERIVED_SEQUENCE};
use crate::{Entity, Store};
use census_domain::model::AccessBlockKind;
use std::cell::Cell;

mod atomicity;
mod bounds;
mod conditional;
mod folding;
mod quota;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
thread_local! { static READS: Cell<u64> = const { Cell::new(0) }; }

pub(super) fn point_read() -> StoreResult<()> {
    READS.with(|reads| {
        reads.set(
            reads
                .get()
                .checked_add(1)
                .ok_or(StoreError::CounterOverflow)?,
        );
        Ok(())
    })
}
fn reads() -> u64 {
    READS.with(Cell::get)
}
fn reset_reads() {
    READS.with(|reads| reads.set(0));
}
fn condition(host: &str, observed: &str, detail: &str) -> SourceAccessCondition {
    SourceAccessCondition::new(
        "fixture",
        host,
        AccessBlockKind::Timeout,
        503,
        observed,
        detail,
    )
}
fn commit(store: &Store, rows: &[SourceAccessCondition]) -> StoreResult<()> {
    let mut batch = store.write_batch();
    batch.merge_source_access(rows)?;
    batch.commit()
}
fn raw(store: &Store, id: &str) -> TestResult<Vec<u8>> {
    Ok(store
        .entities
        .get(observation_key(Table::SourceAccess, id, DERIVED_SEQUENCE))?
        .ok_or("access row absent")?
        .to_vec())
}
fn held(store: &Store, id: &str) -> TestResult<SourceAccessCondition> {
    Ok(serde_json::from_slice(&raw(store, id)?)?)
}
fn exact_record(bytes: usize) -> TestResult<SourceAccessCondition> {
    let mut row = condition("budget", "2026-10-14T00:00:00Z", "");
    let base = ObservationBudget::record_bytes(&row)?;
    row.detail = "x".repeat(
        bytes
            .checked_sub(base)
            .ok_or("record smaller than fixture")?,
    );
    check!(eq; serde_json::to_vec(&row)?.len(), bytes);
    Ok(row)
}
