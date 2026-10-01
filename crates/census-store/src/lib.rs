#![forbid(unsafe_code)]

use fjall::{Database, Keyspace, KeyspaceCreateOptions, PersistMode};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const CACHE_BYTES: u64 = 1024 * 1024 * 1024;

const DB_DIR: &str = "fjall";
const ENTITIES: &str = "entities";
const JOURNAL: &str = "journal";
const META: &str = "meta";
const RECEIPTS: &str = "receipts";

mod backup;
mod batch;
pub mod clock;
mod entities;
mod error;
mod fence;
mod identity;
mod keys;
pub mod read;
mod receipt;
mod rows;
mod sequences;
mod table;
mod write;
mod write_batch;

pub use backup::{BackupReport, IntegrityReport, IntegrityTable, RestoreReport};
pub use error::{StoreError, StoreResult};
pub use fence::FencedSnapshot;
pub use identity::MAX_IDENTITY_APPLICATION_BATCH;
pub use read::{build_athlete_identity_projection, StoreSnapshot};
pub use receipt::{Application, Pruned, Receipt, MAX_DIGEST_BYTES, MAX_OPERATION_BYTES};
pub use rows::TableWalk;
pub use table::{
    Entity, StorageMode, Table, MAX_ID_BYTES, MAX_JOURNAL_KEY_BYTES, MAX_JOURNAL_VALUE_BYTES,
    MAX_ROWS_PER_TABLE,
};
pub use write_batch::StoreBatch;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Consolidated {
    pub rows: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct StoreStats {
    pub tables: Vec<(String, u64)>,
    pub appended: Vec<(String, u64)>,
    pub observations: u64,
    pub bytes_on_disk: u64,
    pub store_bytes: u64,
}

pub struct Store {
    root: PathBuf,
    db: Database,
    entities: Keyspace,
    journal: Keyspace,
    meta: Keyspace,
    receipts: Keyspace,
    sequences: sequences::Counters,
    appends: Mutex<()>,
}

impl Store {
    pub fn open(root: impl AsRef<Path>) -> StoreResult<Self> {
        let root = root.as_ref().to_path_buf();
        ensure_dirs(&root)?;
        let (db, entities, journal, meta, receipts, sequences) = open_keyspaces(&root)?;

        read::sweep_stale_temporaries(&root)?;

        let store = Self {
            root,
            db,
            entities,
            journal,
            meta,
            receipts,
            sequences,
            appends: Mutex::new(()),
        };
        store.seed_row_marks()?;
        Ok(store)
    }
}
fn ensure_dirs(root: &Path) -> StoreResult<()> {
    for sub in ["http", "out"] {
        let dir = root.join(sub);
        std::fs::create_dir_all(&dir).map_err(|source| StoreError::Io {
            path: dir.clone(),
            source,
        })?;
    }
    Ok(())
}

fn open_keyspaces(
    root: &Path,
) -> StoreResult<(
    Database,
    Keyspace,
    Keyspace,
    Keyspace,
    Keyspace,
    sequences::Counters,
)> {
    let db = Database::builder(root.join(DB_DIR))
        .cache_size(CACHE_BYTES)
        .open()
        .map_err(|source| StoreError::Open { source })?;
    let entities = db
        .keyspace(ENTITIES, KeyspaceCreateOptions::default)
        .map_err(|source| StoreError::Open { source })?;
    let journal = db
        .keyspace(JOURNAL, || {
            KeyspaceCreateOptions::default().expect_point_read_hits(true)
        })
        .map_err(|source| StoreError::Open { source })?;
    let meta = db
        .keyspace(META, || {
            KeyspaceCreateOptions::default().expect_point_read_hits(true)
        })
        .map_err(|source| StoreError::Open { source })?;
    let receipts = db
        .keyspace(RECEIPTS, || {
            KeyspaceCreateOptions::default().expect_point_read_hits(true)
        })
        .map_err(|source| StoreError::Open { source })?;
    let sequences = sequences::Counters::seeded(&db, &entities, &meta)?;
    Ok((db, entities, journal, meta, receipts, sequences))
}

impl Store {
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn http_cache_dir(&self) -> PathBuf {
        self.root.join("http")
    }

    pub fn out_dir(&self) -> PathBuf {
        self.root.join("out")
    }

    pub fn table_path(&self, table: Table) -> PathBuf {
        self.root
            .join("entities")
            .join(format!("{}.jsonl", table.file()))
    }

    pub fn flush(&self) -> StoreResult<()> {
        self.db
            .persist(PersistMode::SyncAll)
            .map_err(|source| StoreError::Flush { source })
    }
    pub fn snapshot(&self) -> StoreSnapshot<'_> {
        StoreSnapshot::new(self.db.snapshot(), &self.entities, &self.root)
    }
}
#[cfg(test)]
#[path = "backup_tests.rs"]
mod backup_tests;
#[cfg(all(feature = "loom", test))]
mod loom_tests;
#[cfg(test)]
mod marks_tests;
#[cfg(test)]
mod receipt_tests;
#[cfg(test)]
mod replace_tests;
#[cfg(test)]
mod tests;
#[cfg(kani)]
include!("../kani/store_wiring.rs");
