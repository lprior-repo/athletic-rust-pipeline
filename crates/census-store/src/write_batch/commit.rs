use fjall::{OwnedWriteBatch, PersistMode};

use super::super::batch::stage_derived_encoded;
use super::super::keys::observation_key;
use super::super::receipt::{self, Application, Decision};
use super::super::sequences::Reserved;
use super::super::{generation, Store, StoreError, StoreResult};
use super::{conditional, ConditionalBatch, Page, Replacement, StoreBatch};

impl<'store> StoreBatch<'store> {
    pub fn commit(self) -> StoreResult<()> {
        self.commit_inner(None, None, Vec::new()).map(|_| ())
    }

    pub fn commit_once(self, operation: &str, digest: &str) -> StoreResult<Application> {
        receipt::refuse_over_operation(operation, digest)?;
        self.commit_inner(Some((operation, digest)), None, Vec::new())?
            .ok_or_else(|| StoreError::Invariant {
                detail: format!(
                    "receipted commit of operation {operation} wrote neither rows nor a receipt"
                ),
            })
    }

    pub fn commit_once_at_evidence_generation(
        self,
        operation: &str,
        digest: &str,
        generation: u64,
    ) -> StoreResult<Application> {
        receipt::refuse_over_operation(operation, digest)?;
        self.commit_inner(Some((operation, digest)), Some(generation), Vec::new())?
            .ok_or_else(|| StoreError::Invariant {
                detail: format!("fenced operation {operation} wrote neither rows nor a receipt"),
            })
    }

    pub fn commit_once_with_effects(
        self,
        operation: &str,
        digest: &str,
        effects: Vec<ConditionalBatch<'store, '_>>,
    ) -> StoreResult<Application> {
        receipt::refuse_over_operation(operation, digest)?;
        conditional::admit(&self, &effects, operation)?;
        self.commit_inner(Some((operation, digest)), None, effects)?
            .ok_or_else(|| StoreError::Invariant {
                detail: format!("conditional operation {operation} produced no receipt"),
            })
    }

    fn commit_inner(
        self,
        once: Option<(&str, &str)>,
        evidence: Option<u64>,
        effects: Vec<ConditionalBatch<'store, '_>>,
    ) -> StoreResult<Option<Application>> {
        if self.is_empty() && once.is_none() {
            return Ok(None);
        }
        let store = self.store;
        let _appends = store.lock_appends();
        if let Some((operation, digest)) = once {
            if let Decision::Repeat(receipt) = receipt::decide(store, operation, digest)? {
                return Ok(Some(Application::Repeated(receipt)));
            }
        }
        refuse_moved_evidence(store, once, evidence)?;
        let mut batch = store.db.batch();
        let prepared = conditional::merge(self, effects, &mut batch)?;
        prepared.commit_staged(once, batch)
    }

    fn commit_staged(
        self,
        once: Option<(&str, &str)>,
        mut batch: OwnedWriteBatch,
    ) -> StoreResult<Option<Application>> {
        let store = self.store;
        let (pages, journal, replacements) = (self.pages, self.journal, self.replacements);
        let written = once
            .map(|identity| first_application(store, identity, count_rows(&pages)?, &mut batch))
            .transpose()?;
        let reservations = plan_reservations(&pages, store)?;
        write_pages_staged(&pages, &reservations, store, &mut batch)?;
        write_journal(journal, store, &mut batch);
        let generation = stage_generation(&pages, &replacements, store, &mut batch)?;
        write_replacements(replacements, store, &mut batch)?;
        #[cfg(feature = "native-fault-injection")]
        if let Some(Application::Written(receipt)) = &written {
            let ordinal = store
                .receipt_count()?
                .checked_add(1)
                .ok_or(StoreError::CounterOverflow)?;
            crate::native_worker_boundary::pause(
                crate::native_worker_boundary::Position::SourceBatchStagedBeforeCommit { ordinal },
                &receipt.operation,
                &receipt.digest,
            )?;
        }
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        publish_marks(&pages, &reservations, generation, store)?;
        #[cfg(feature = "native-fault-injection")]
        if let Some(Application::Written(receipt)) = &written {
            crate::native_worker_boundary::pause(
                crate::native_worker_boundary::Position::SourceChunkCommittedBeforeNext {
                    ordinal: store.receipt_count()?,
                },
                &receipt.operation,
                &receipt.digest,
            )?;
        }
        Ok(written)
    }
}

fn stage_generation(
    pages: &[Page],
    replacements: &[Replacement],
    store: &Store,
    batch: &mut OwnedWriteBatch,
) -> StoreResult<Option<u64>> {
    if pages.is_empty() && replacements.is_empty() {
        return Ok(None);
    }
    let generation = store.generations.next_entities()?;
    generation::write_entities(batch, &store.meta, generation);
    Ok(Some(generation))
}

fn refuse_moved_evidence(
    store: &Store,
    once: Option<(&str, &str)>,
    evidence: Option<u64>,
) -> StoreResult<()> {
    let Some(expected) = evidence else {
        return Ok(());
    };
    let actual = store.evidence_generation();
    if actual == expected {
        return Ok(());
    }
    Err(StoreError::EvidenceMoved {
        operation: once.map_or_else(String::new, |(operation, _)| operation.to_string()),
        expected,
        actual,
    })
}

fn plan_reservations(pages: &[Page], store: &Store) -> StoreResult<Vec<Reserved>> {
    pages
        .iter()
        .map(|page| {
            let count =
                u64::try_from(page.records.len()).map_err(|_| StoreError::CounterOverflow)?;
            store.sequences.plan(page.table, count)
        })
        .collect()
}

fn publish_marks(
    pages: &[Page],
    reservations: &[Reserved],
    generation: Option<u64>,
    store: &Store,
) -> StoreResult<()> {
    pages
        .iter()
        .zip(reservations)
        .try_for_each(|(page, reservation)| {
            store.sequences.publish(page.table, reservation.mark)
        })?;
    if let Some(generation) = generation {
        store.generations.publish_entities(generation);
    }
    Ok(())
}

pub(super) fn count_rows(pages: &[Page]) -> StoreResult<u64> {
    pages.iter().try_fold(0_u64, |total, page| {
        let count = u64::try_from(page.records.len()).map_err(|_| StoreError::CounterOverflow)?;
        total.checked_add(count).ok_or(StoreError::CounterOverflow)
    })
}

pub(super) fn first_application(
    store: &Store,
    identity: (&str, &str),
    appended: u64,
    batch: &mut OwnedWriteBatch,
) -> StoreResult<Application> {
    let receipt = receipt::first(identity.0, identity.1, appended);
    let value = serde_json::to_vec(&receipt).map_err(|source| StoreError::Json {
        detail: "serializing a receipt".to_string(),
        source,
    })?;
    batch.insert(&store.receipts, identity.0.as_bytes(), value);
    Ok(Application::Written(receipt))
}

fn write_pages_staged(
    pages: &[Page],
    reservations: &[Reserved],
    store: &Store,
    batch: &mut OwnedWriteBatch,
) -> StoreResult<()> {
    for (page, reservation) in pages.iter().zip(reservations.iter()) {
        let count = u64::try_from(page.records.len()).map_err(|_| StoreError::CounterOverflow)?;
        for (offset, (id, value)) in page.records.iter().enumerate() {
            let offset = u64::try_from(offset).map_err(|_| StoreError::CounterOverflow)?;
            let sequence = reservation
                .base
                .checked_add(offset)
                .ok_or(StoreError::CounterOverflow)?;
            batch.insert(
                &store.entities,
                observation_key(page.table, id, sequence),
                value,
            );
        }
        store.put_mark(batch, page.table, reservation.mark);
        let rows = store
            .count(page.table)?
            .checked_add(count)
            .ok_or(StoreError::CounterOverflow)?;
        store.put_row_mark(batch, page.table, rows);
    }
    Ok(())
}

fn write_journal(journal: Vec<(Vec<u8>, Vec<u8>)>, store: &Store, batch: &mut OwnedWriteBatch) {
    for (key, value) in journal {
        batch.insert(&store.journal, key, value);
    }
}

fn write_replacements(
    replacements: Vec<Replacement>,
    store: &Store,
    batch: &mut OwnedWriteBatch,
) -> StoreResult<()> {
    let generation = store.derived_generation();
    for replacement in replacements {
        let staged = stage_derived_encoded(
            batch,
            &store.entities,
            replacement.table,
            replacement.records,
            generation,
        )?;
        let held = store.count(replacement.table)?;
        let rows = held
            .checked_add(staged.added)
            .ok_or(StoreError::CounterOverflow)?;
        store.put_row_mark(batch, replacement.table, rows);
    }
    Ok(())
}
