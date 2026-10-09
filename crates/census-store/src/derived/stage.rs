use std::sync::MutexGuard;

use fjall::PersistMode;
use serde::Serialize;

use super::super::keys::{
    derived_generation_prefix, derived_key, observation_id, view_derived_key,
};
use super::super::read::StoreSnapshot;
use super::super::{
    generation, receipt, rows, Application, Receipt, Store, StoreError, StoreResult, Table,
};
use super::reclaim::{self, Reclaimed};

const MAX_STAGE_ROWS: usize = 25_000;
const MAX_STAGE_BYTES: usize = 64 * 1024 * 1024;
const PUBLICATION_RECLAIM_BUDGET: u64 = 250_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publication {
    pub application: Application,
    pub generation: u64,
    pub reclaimed: Reclaimed,
}

pub struct DerivedStage<'s> {
    store: &'s Store,
    exclusivity: Option<MutexGuard<'s, ()>>,
    generation: u64,
    base_evidence: u64,
    base_derived_generation: u64,
    snapshot: StoreSnapshot<'s>,
    buffered: Vec<(Vec<u8>, Vec<u8>)>,
    buffered_bytes: usize,
    tables: Vec<Table>,
}

fn reclaim_budgeted(store: &Store) -> Reclaimed {
    match reclaim::reclaim(store, PUBLICATION_RECLAIM_BUDGET) {
        Ok(reclaimed) => reclaimed,
        Err(error) => {
            tracing::warn!(
                %error,
                "post-publication reclaim did not finish; the next publication retries it"
            );
            Reclaimed::default()
        }
    }
}

impl<'s> DerivedStage<'s> {
    pub(super) fn begin(store: &'s Store) -> StoreResult<Self> {
        let exclusivity = store.lock_staging();
        let generation = generation::reserve_derived(&store.db, &store.meta)?;
        let snapshot = store.snapshot();
        let base_evidence = snapshot.evidence_generation();
        let base_derived_generation = snapshot.derived_generation();
        Ok(Self {
            store,
            exclusivity: Some(exclusivity),
            generation,
            base_evidence,
            base_derived_generation,
            snapshot,
            buffered: Vec::new(),
            buffered_bytes: 0,
            tables: Vec::new(),
        })
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn view(&self) -> &StoreSnapshot<'s> {
        &self.snapshot
    }

    pub fn replace_many<T: Serialize>(&mut self, table: Table, records: &[T]) -> StoreResult<()> {
        self.refuse_unpartitioned(table)?;
        self.stage_table(table);
        for record in records {
            let value = serde_json::to_vec(record).map_err(|source| StoreError::Json {
                detail: format!("table {} record is not encodable", table.file()),
                source,
            })?;
            let id = observation_id(&value)?;
            self.buffered_bytes = self.buffered_bytes.saturating_add(value.len());
            self.buffered
                .push((derived_key(table, self.generation, id.as_bytes()), value));
            if self.over_budget() {
                self.flush()?;
            }
        }
        Ok(())
    }

    pub fn carry_forward(&mut self, table: Table) -> StoreResult<u64> {
        self.refuse_unpartitioned(table)?;
        self.stage_table(table);
        let prefix = derived_generation_prefix(table, self.base_derived_generation);
        let mut moved = 0_u64;
        for guard in self.store.entities.prefix(prefix.as_slice()) {
            let (key, value) = guard
                .into_inner()
                .map_err(|source| StoreError::Read { source })?;
            let (_, id) = view_derived_key(table, &key).ok_or_else(|| StoreError::Invariant {
                detail: format!(
                    "table {} holds a malformed derived key during carry-forward",
                    table.file()
                ),
            })?;
            self.buffered_bytes = self.buffered_bytes.saturating_add(value.len());
            self.buffered
                .push((derived_key(table, self.generation, id), value.to_vec()));
            moved = moved.checked_add(1).ok_or(StoreError::CounterOverflow)?;
            if self.over_budget() {
                self.flush()?;
            }
        }
        Ok(moved)
    }

    pub fn publish(mut self, operation: &str, digest: &str) -> StoreResult<Publication> {
        receipt::refuse_over_operation(operation, digest)?;
        self.flush()?;
        #[cfg(feature = "native-fault-injection")]
        crate::native_worker_boundary::pause(
            crate::native_worker_boundary::Position::DerivedBatchStagedBeforePublish {
                generation: self.generation,
            },
            operation,
            digest,
        )?;
        let exclusivity = self.exclusivity.take();
        let _appends = self.store.lock_appends();
        self.refuse_moved_inputs()?;
        if let receipt::Decision::Repeat(receipt) = receipt::decide(self.store, operation, digest)?
        {
            drop(_appends);
            drop(exclusivity);
            return Ok(Publication {
                application: Application::Repeated(receipt),
                generation: self.generation,
                reclaimed: reclaim_budgeted(self.store),
            });
        }
        let written = receipt::first(operation, digest, 0);
        self.write_publication(operation, &written)?;
        drop(_appends);
        drop(exclusivity);
        Ok(Publication {
            application: Application::Written(written),
            generation: self.generation,
            reclaimed: reclaim_budgeted(self.store),
        })
    }

    fn refuse_moved_inputs(&self) -> StoreResult<()> {
        let evidence = self.store.evidence_generation();
        let derived = self.store.derived_generation();
        if evidence == self.base_evidence && derived == self.base_derived_generation {
            return Ok(());
        }
        Err(StoreError::PublicationRefused {
            detail: format!(
                "inputs moved from generation {} to {} and the derived pointer from {} to {} \
                 while generation {} was staged",
                self.base_evidence,
                evidence,
                self.base_derived_generation,
                derived,
                self.generation
            ),
        })
    }

    fn write_publication(&self, operation: &str, receipt: &Receipt) -> StoreResult<()> {
        let mut batch = self.store.db.batch();
        for table in &self.tables {
            let held = rows::count_generation(&self.store.entities, *table, self.generation)?;
            self.store.put_row_mark(&mut batch, *table, held);
        }
        let next_entities = self.store.generations.next_entities()?;
        generation::write_entities(&mut batch, &self.store.meta, next_entities);
        generation::write_derived_current(&mut batch, &self.store.meta, self.generation);
        batch.insert(
            &self.store.receipts,
            operation.as_bytes(),
            serde_json::to_vec(receipt).map_err(|source| StoreError::Json {
                detail: format!("receipt for {operation} is not encodable"),
                source,
            })?,
        );
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        self.store.generations.publish_entities(next_entities);
        self.store
            .generations
            .publish_derived_current(self.generation);
        Ok(())
    }

    fn refuse_unpartitioned(&self, table: Table) -> StoreResult<()> {
        if table.generation_partitioned() {
            return Ok(());
        }
        Err(StoreError::NotGenerationTable {
            table: table.file(),
        })
    }

    fn stage_table(&mut self, table: Table) {
        if !self.tables.contains(&table) {
            self.tables.push(table);
        }
    }

    fn over_budget(&self) -> bool {
        self.buffered.len() >= MAX_STAGE_ROWS || self.buffered_bytes >= MAX_STAGE_BYTES
    }

    fn flush(&mut self) -> StoreResult<()> {
        if self.buffered.is_empty() {
            return Ok(());
        }
        let mut batch = self.store.db.batch();
        for (key, value) in self.buffered.drain(..) {
            batch.insert(&self.store.entities, key, value);
        }
        self.buffered_bytes = 0;
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })
    }
}
