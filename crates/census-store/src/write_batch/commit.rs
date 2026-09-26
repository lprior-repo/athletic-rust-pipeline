use fjall::{OwnedWriteBatch, PersistMode};

use super::super::batch::{drop_unnamed, refuse_over_bound, stage_derived_encoded};
use super::super::keys::observation_key;
use super::super::receipt::{self, Application, Decision};
use super::super::{StorageMode, Store, StoreError, StoreResult};
use super::{Page, Replacement, StoreBatch};

impl StoreBatch<'_> {
    /// Commit every buffered append and journal entry as one batch, one durability boundary.
    pub fn commit(self) -> StoreResult<()> {
        self.commit_inner(None).map(|_| ())
    }

    /// Commit the batch as one application of `operation`, whose payload digests to `digest`.
    ///
    /// The receipt reaches the database in the same commit as the rows and journal entries it
    /// names — so the store never holds rows no receipt covers, which is the state a crash between
    /// two commits would leave and the state a replay appends again. A commit that finds the
    /// operation already applied under the same digest writes nothing at all — no row, no counter
    /// movement, no journal entry — and answers [`Application::Repeated`]; one that finds the id
    /// applied under a different digest is refused, because the same id cannot name two payloads.
    ///
    /// The id, the digest and what the store already holds are checked inside the append lock, so
    /// two callers offering one operation cannot both read an absent receipt and both write one.
    pub fn commit_once(self, operation: &str, digest: &str) -> StoreResult<Application> {
        receipt::refuse_over_operation(operation, digest)?;
        self.commit_inner(Some((operation, digest)))?
            .ok_or_else(|| StoreError::Invariant {
                detail: format!(
                    "receipted commit of operation {operation} wrote neither rows nor a receipt"
                ),
            })
    }

    /// The one commit path: `once` carries the operation a receipted commit records.
    ///
    /// The reservation, the batch and the commit are one critical section of the append lock, for
    /// the reason `Store::append_many` states: the writer that reserves first commits first, so no
    /// batch can leave a table's mark below observations an earlier batch stored above it.
    ///
    /// `Ok(None)` means the batch held nothing to write, which only an unreceipted batch can be.
    fn commit_inner(self, once: Option<(&str, &str)>) -> StoreResult<Option<Application>> {
        if self.is_empty() && once.is_none() {
            return Ok(None);
        }
        let store = self.store;
        let _appends = store.lock_appends();
        let (pages, journal, replacements) = (self.pages, self.journal, self.replacements);
        let mut batch = store.db.batch();
        let written = prepare_receipt(&pages, store, once, &mut batch)?;
        if matches!(written, Some(Application::Repeated(_))) {
            return Ok(written);
        }
        check_bound(&pages, store)?;
        write_pages(pages, store, &mut batch)?;
        write_journal(journal, store, &mut batch);
        write_replacements(replacements, store, &mut batch)?;
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        Ok(written)
    }
}

/// Check that no page would exceed the table's row ceiling once its reservation moves.
fn check_bound(pages: &[Page], store: &Store) -> StoreResult<()> {
    for page in pages {
        let count = u64::try_from(page.records.len()).map_err(|_| StoreError::CounterOverflow)?;
        let reached = store
            .sequences
            .next_sequence(page.table)
            .checked_add(count)
            .ok_or(StoreError::CounterOverflow)?;
        refuse_over_bound(page.table, reached)?;
    }
    Ok(())
}

/// Decide the receipt a receipted commit records, and buffer it on `batch`.
///
/// Returns `Some(Application::Repeated(standing))` when the operation is a replay,
/// `Some(Application::Written(receipt))` when a fresh receipt is buffered, and `None` for an
/// unreceipted commit — which writes no receipt at all.
fn prepare_receipt(
    pages: &[Page],
    store: &Store,
    once: Option<(&str, &str)>,
    batch: &mut OwnedWriteBatch,
) -> StoreResult<Option<Application>> {
    if let Some((operation, digest)) = once {
        if let Decision::Repeat(standing) = receipt::decide(store, operation, digest)? {
            return Ok(Some(Application::Repeated(standing)));
        }
    }
    match once {
        None => Ok(None),
        Some((operation, digest)) => {
            let mut appended = 0_u64;
            for page in pages {
                let count =
                    u64::try_from(page.records.len()).map_err(|_| StoreError::CounterOverflow)?;
                appended = appended
                    .checked_add(count)
                    .ok_or(StoreError::CounterOverflow)?;
            }
            let receipt = receipt::first(operation, digest, appended);
            let value = serde_json::to_vec(&receipt).map_err(|source| StoreError::Json {
                detail: "serializing a receipt".to_string(),
                source,
            })?;
            batch.insert(&store.receipts, operation.as_bytes(), value);
            Ok(Some(Application::Written(receipt)))
        }
    }
}

/// Write all buffered pages into `batch`, consuming the page list.
fn write_pages(pages: Vec<Page>, store: &Store, batch: &mut OwnedWriteBatch) -> StoreResult<()> {
    for page in pages {
        let table = page.table;
        let count = u64::try_from(page.records.len()).map_err(|_| StoreError::CounterOverflow)?;
        let reserved = store.reserve(table, count)?;
        for (offset, (id, value)) in page.records.into_iter().enumerate() {
            let offset = u64::try_from(offset).map_err(|_| StoreError::CounterOverflow)?;
            let sequence = reserved
                .base
                .checked_add(offset)
                .ok_or(StoreError::CounterOverflow)?;
            batch.insert(
                &store.entities,
                observation_key(table, &id, sequence),
                value,
            );
        }
        store.put_mark(batch, table, reserved.mark);
        let rows = store.count(table)?.saturating_add(count);
        store.put_row_mark(batch, table, rows);
    }
    Ok(())
}

/// Write all buffered journal entries into `batch`.
fn write_journal(journal: Vec<(Vec<u8>, Vec<u8>)>, store: &Store, batch: &mut OwnedWriteBatch) {
    for (key, value) in journal {
        batch.insert(&store.journal, key, value);
    }
}

/// Write all buffered replacements into `batch`, with each table's derived row count.
fn write_replacements(
    replacements: Vec<Replacement>,
    store: &Store,
    batch: &mut OwnedWriteBatch,
) -> StoreResult<()> {
    for replacement in replacements {
        let staged = stage_derived_encoded(
            batch,
            &store.entities,
            replacement.table,
            replacement.records,
        )?;
        if replacement.table.storage_mode() == StorageMode::DerivedSnapshot {
            drop_unnamed(&store.entities, batch, replacement.table, &staged.named)?;
        }
        let held = store.count(replacement.table)?;
        let rows = match replacement.table.storage_mode() {
            StorageMode::DerivedSnapshot => staged.named_count()?,
            StorageMode::DerivedMap | StorageMode::ObservationLog => {
                held.saturating_add(staged.added)
            }
        };
        store.put_row_mark(batch, replacement.table, rows);
    }
    Ok(())
}
