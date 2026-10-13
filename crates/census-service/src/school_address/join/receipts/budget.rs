use census_store::{
    ObservationBudget, Receipt, StoreError, StoreResult, Table, MAX_JOURNAL_VALUE_BYTES,
    MAX_OBSERVATION_BATCH_BYTES, MAX_OBSERVATION_BATCH_ROWS,
};
use serde::Serialize;

use super::WITNESS_PHASE;

const MAX_TARGET_STAGED_BYTES: usize = MAX_OBSERVATION_BATCH_BYTES;
const U64_MAX_TEXT: &str = "18446744073709551615";
const MAX_TIMESTAMP: &str = "0000000000000000000000000000000000000000000000000000000000000000";
const MAX_DATE: &str = "00000000000000000000000000000000";

#[derive(Default)]
pub(super) struct TargetBudget {
    bytes: usize,
    rows: usize,
    schools: bool,
    cases: bool,
}

impl TargetBudget {
    pub(super) fn admit_row<T: Serialize + ?Sized>(
        &mut self,
        table: Table,
        value: &T,
        id: &str,
    ) -> StoreResult<()> {
        let rows = self
            .rows
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        check(
            "school-address target rows",
            rows,
            MAX_OBSERVATION_BATCH_ROWS,
        )?;
        let bytes = ObservationBudget::record_bytes(value)?
            .checked_add(row_key_bytes(table, id)?)
            .ok_or(StoreError::CounterOverflow)?;
        self.admit_bytes(bytes)?;
        self.rows = rows;
        self.schools |= table == Table::Schools;
        self.cases |= table == Table::ReviewCases;
        Ok(())
    }

    pub(super) fn admit_bytes(&mut self, bytes: usize) -> StoreResult<()> {
        let total = self
            .bytes
            .checked_add(bytes)
            .ok_or(StoreError::CounterOverflow)?;
        check(
            "school-address target staged bytes",
            total,
            MAX_TARGET_STAGED_BYTES,
        )?;
        self.bytes = total;
        Ok(())
    }

    pub(super) fn admit_witness<T: Serialize>(&self, witness: &T) -> StoreResult<()> {
        let bytes = self
            .bytes
            .checked_add(ObservationBudget::record_bytes(witness)?)
            .ok_or(StoreError::CounterOverflow)?;
        check(
            "school-address target staged bytes",
            bytes,
            MAX_TARGET_STAGED_BYTES,
        )
    }

    pub(super) fn admit_application<T: Serialize>(
        &self,
        witness: &T,
        operation: &str,
        digest: &str,
    ) -> StoreResult<()> {
        let envelope = JournalEnvelopeUpperBound {
            key: operation,
            at: MAX_TIMESTAMP,
            payload: witness,
        };
        let envelope_bytes = ObservationBudget::record_bytes(&envelope)?;
        check(
            "school-address witness journal bytes",
            envelope_bytes,
            MAX_JOURNAL_VALUE_BYTES,
        )?;
        let receipt = Receipt {
            operation: super::copy_string(operation)?,
            digest: super::copy_string(digest)?,
            at: super::copy_string(MAX_DATE)?,
            appended: u64::MAX,
            credited: false,
        };
        let receipt_bytes = ObservationBudget::record_bytes(&receipt)?;
        let journal_key_bytes = WITNESS_PHASE
            .len()
            .checked_add(1)
            .and_then(|bytes| bytes.checked_add(operation.len()))
            .ok_or(StoreError::CounterOverflow)?;
        let bytes = [
            self.bytes,
            envelope_bytes,
            receipt_bytes,
            journal_key_bytes,
            operation.len(),
            self.metadata_bytes()?,
        ]
        .into_iter()
        .try_fold(0usize, add_bytes)?;
        check(
            "school-address target staged bytes",
            bytes,
            MAX_TARGET_STAGED_BYTES,
        )
    }
    fn metadata_bytes(&self) -> StoreResult<usize> {
        let mut bytes = mark_bytes("generation:entities", None)?;
        if self.schools {
            bytes = add_bytes(bytes, mark_bytes("sequence:", Some(Table::Schools))?)?;
            bytes = add_bytes(bytes, mark_bytes("rows:", Some(Table::Schools))?)?;
        }
        if self.cases {
            bytes = add_bytes(bytes, mark_bytes("rows:", Some(Table::ReviewCases))?)?;
        }
        Ok(bytes)
    }
}

#[derive(Serialize)]
struct JournalEnvelopeUpperBound<'a, T> {
    key: &'a str,
    at: &'static str,
    payload: &'a T,
}

pub(super) fn check(what: &'static str, needed: usize, available: usize) -> StoreResult<()> {
    if needed <= available {
        return Ok(());
    }
    Err(StoreError::BudgetExceeded {
        what,
        needed: u64::try_from(needed).map_err(|_| StoreError::CounterOverflow)?,
        available: u64::try_from(available).map_err(|_| StoreError::CounterOverflow)?,
    })
}

fn row_key_bytes(table: Table, id: &str) -> StoreResult<usize> {
    let prefix = match table {
        Table::Schools => add_bytes(table.file().len(), 2)?,
        Table::ReviewCases => add_bytes(add_bytes("derived".len(), table.file().len())?, 3)?,
        _ => {
            return Err(StoreError::Invariant {
                detail: "school-address application budgets only Schools and ReviewCases".into(),
            })
        }
    };
    add_bytes(add_bytes(prefix, std::mem::size_of::<u64>())?, id.len())
}

fn mark_bytes(prefix: &str, table: Option<Table>) -> StoreResult<usize> {
    let key_bytes = add_bytes(prefix.len(), table.map_or(0, |table| table.file().len()))?;
    add_bytes(key_bytes, U64_MAX_TEXT.len())
}

fn add_bytes(held: usize, bytes: usize) -> StoreResult<usize> {
    held.checked_add(bytes).ok_or(StoreError::CounterOverflow)
}
