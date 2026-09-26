use serde::Serialize;

use super::super::batch::refuse_over_bound;
use super::super::keys::observation_id;
use super::super::{StorageMode, StoreError, StoreResult, Table};
use super::{Page, Replacement, StoreBatch};

impl StoreBatch<'_> {
    /// Buffer records for one table.
    ///
    /// Every record is serialized and keyed here, so an unencodable one refuses this call rather than
    /// the commit, and pages for the same table are kept as one.
    pub fn append_many<T: Serialize>(&mut self, table: Table, records: &[T]) -> StoreResult<()> {
        if records.is_empty() {
            return Ok(());
        }
        let mut encoded = Vec::with_capacity(records.len());
        for record in records {
            let value = serde_json::to_vec(record).map_err(|source| StoreError::Json {
                detail: "serializing a batched observation".to_string(),
                source,
            })?;
            encoded.push((observation_id(&value)?.to_string(), value));
        }
        match self.pages.iter_mut().find(|page| page.table == table) {
            Some(page) => page.records.extend(encoded),
            None => self.pages.push(Page {
                table,
                records: encoded,
            }),
        }
        Ok(())
    }

    /// Stage `records` as the whole content of a derived `table`, inside this commit.
    ///
    /// [`Store::replace_many`](crate::Store::replace_many) writes one table's content in one commit
    /// of its own. A pass that derives
    /// two tables at once cannot use it twice: a crash between the two calls leaves verdicts recorded
    /// against cases that still read as open. Staged here instead, both tables reach the database in the
    /// caller's own commit — one durability boundary, and one receipt when the caller also names the
    /// operation, so a replay of the pass leaves the pair as the first application left it.
    ///
    /// The rules are the table's own, exactly as the single-table write applies them: a snapshot table
    /// (or a caller passing no records for one) comes out holding only what the call names, and a
    /// never-committed batch leaves it untouched. A table cannot be both appended and replaced in one
    /// batch, because the two disagree about what the table holds.
    pub fn replace_many<T: Serialize>(&mut self, table: Table, records: &[T]) -> StoreResult<()> {
        if records.is_empty() && table.storage_mode() != StorageMode::DerivedSnapshot {
            return Ok(());
        }
        if self.pages.iter().any(|page| page.table == table) {
            return Err(StoreError::Invariant {
                detail: format!(
                    "table {} is appended and replaced in one batch",
                    table.file()
                ),
            });
        }
        if self.replacements.iter().any(|held| held.table == table) {
            return Err(StoreError::Invariant {
                detail: format!("table {} is replaced twice in one batch", table.file()),
            });
        }
        let count = u64::try_from(records.len()).map_err(|_| StoreError::CounterOverflow)?;
        refuse_over_bound(table, count)?;
        let mut encoded = Vec::with_capacity(records.len());
        for record in records {
            let value = serde_json::to_vec(record).map_err(|source| StoreError::Json {
                detail: "serializing a derived record".to_string(),
                source,
            })?;
            encoded.push(value);
        }
        self.replacements.push(Replacement {
            table,
            records: encoded,
        });
        Ok(())
    }
}
