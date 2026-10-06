use fjall::{OwnedWriteBatch, PersistMode};

use super::super::batch::stage_derived_encoded;
use super::super::keys::observation_key;
use super::super::receipt::{self, Application, Decision};
use super::super::sequences::Reserved;
use super::super::{generation, Store, StoreError, StoreResult};
use super::{Page, Replacement, StoreBatch};

impl StoreBatch<'_> {
    pub fn commit(self) -> StoreResult<()> {
        self.commit_inner(None, None).map(|_| ())
    }

    pub fn commit_once(self, operation: &str, digest: &str) -> StoreResult<Application> {
        receipt::refuse_over_operation(operation, digest)?;
        self.commit_inner(Some((operation, digest)), None)?
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
        self.commit_inner(Some((operation, digest)), Some(generation))?
            .ok_or_else(|| StoreError::Invariant {
                detail: format!("fenced operation {operation} wrote neither rows nor a receipt"),
            })
    }

    fn commit_inner(
        self,
        once: Option<(&str, &str)>,
        evidence: Option<u64>,
    ) -> StoreResult<Option<Application>> {
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
        refuse_moved_evidence(store, once, evidence)?;
        let reservations = plan_reservations(&pages, store)?;
        write_pages_staged(&pages, &reservations, store, &mut batch)?;
        write_journal(journal, store, &mut batch);
        let writes_entities = !pages.is_empty() || !replacements.is_empty();
        write_replacements(replacements, store, &mut batch)?;
        let generation = if writes_entities {
            let generation = store.generations.next_entities()?;
            generation::write_entities(&mut batch, &store.meta, generation);
            Some(generation)
        } else {
            None
        };
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        publish_marks(&pages, &reservations, generation, store)?;
        Ok(written)
    }
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
        store.put_row_mark(batch, replacement.table, held.saturating_add(staged.added));
    }
    Ok(())
}
