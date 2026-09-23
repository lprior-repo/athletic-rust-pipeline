//! Fjall-backed entity store: append-only observations, deduplicated snapshots, resume journal.
//!
//! Collection is interrupted constantly (politeness delays, network, operator), so every adapter
//! appends observations instead of rewriting state. The substrate is [Fjall](https://fjall-rs.github.io),
//! an embedded LSM-tree key-value store in safe Rust: writes land in a write-ahead journal and a
//! memtable, and are compacted into immutable sorted tables, so an interrupted run costs at most the
//! observations that were never flushed — never a rewritten snapshot.
//!
//! # Keyspace layout
//!
//! ```text
//! entities: <table>\0<entity-id>\0<sequence:u64 big-endian>   -> observation JSON
//! journal:  <phase>\0<key>                                    -> {key, at, payload}
//! meta:     <name>                                            -> small JSON/scalar
//! meta:     sequence:<table>                                  -> next observation sequence
//! ```
//!
//! Observations are append-only: appending the same entity twice writes two rows, and
//! [`Store::consolidate`] merges them through [`Entity::merge`], which is exactly the guarantee the
//! JSONL journals used to provide. The sequence component is **big-endian** so byte order is
//! numerical order, and it is seeded from the table's *mark* — the `sequence:<table>` row its last
//! append committed in the same batch as the observations that spent the sequences — so reopening a
//! database never reuses a sequence number, never overwrites an observation, and never walks a table
//! to find out where to resume.
//!
//! # Marks
//!
//! A table's mark is the sequence its next append will use, so it is also the number a reopen seeds
//! that table's counter from. It is written by the batch it accounts for, which is what keeps the two
//! from disagreeing, and the writers that advance it commit in reservation order (see
//! `Store::lock_appends`), which is what keeps it from ever moving backwards.
//!
//! A database written before marks existed holds none. The open that misses one derives it with the
//! one scan this store has always done, commits the result in one durable batch, and is the last open
//! that reads that table: see [`sequences::Counters::seeded`]. A mark that is present but unreadable
//! fails the open with [`StoreError::Invariant`] rather than being guessed at.
//!
//! # Durability
//!
//! Batches are committed to the journal with [`PersistMode::SyncData`] (`fdatasync`), which is the
//! cheapest mode that survives a machine crash. [`Store::flush`] upgrades this to
//! [`PersistMode::SyncAll`] and is called at consolidation and at shutdown. A lost tail costs
//! re-running an adapter, and the resume journal is durable per completed unit of work, so a
//! resumed run does not repeat finished work.
//!
//! # Legacy journals
//!
//! Databases created before the Fjall substrate keep their rows in `<store>/entities/*.jsonl` and
//! their resume ledger in `<store>/journal/*.jsonl`. [`Store::import_legacy`] imports both exactly
//! once (recorded under `meta`), skipping the import when the marker is present, so a partially
//! imported database finishes importing with the next call without duplicating observations.
//!
//! Opening a store does not import: [`Store::open`] is a read, so a verb that only measures a legacy
//! root — a status, an integrity check, a backup — does not migrate the corpus as a side effect. The
//! paths that have decided to migrate call [`Store::import_legacy`] themselves: the offline census
//! run, the `import-legacy` verb, and the service bootstrap that owns the store for the live route.

use fjall::{Database, Keyspace, KeyspaceCreateOptions, PersistMode};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Unified cache for the LSM tree. Bounded on purpose: the default is sized to the machine, and this
/// process is expected to share the machine with a browser and a text editor.
const CACHE_BYTES: u64 = 256 * 1024 * 1024;

const DB_DIR: &str = "fjall";
const ENTITIES: &str = "entities";
const JOURNAL: &str = "journal";
const META: &str = "meta";

mod backup;
mod batch;
mod error;
mod entities;
mod keys;
mod legacy;
pub mod read;
mod rows;
mod sequences;
mod table;
mod write;

pub use backup::{BackupReport, IntegrityReport, IntegrityTable, RestoreReport};
pub use error::{StoreError, StoreResult};
pub use rows::TableWalk;
pub use table::{
    Entity, StorageMode, Table, MAX_ID_BYTES, MAX_JOURNAL_KEY_BYTES, MAX_JOURNAL_VALUE_BYTES,
    MAX_ROWS_PER_TABLE,
};

/// What one [`Store::consolidate`] call produced: the rows written, and how many of them the
/// collection contract withheld a consumer mailbox from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Consolidated {
    pub rows: usize,
    pub withheld: usize,
}

/// Per-table rows and the database's on-disk footprint. The figures come from the store's own durable
/// ledger — exact counts rather than LSM `approximate_len` estimates — so a status command reports
/// what the store holds without scanning millions of rows.
///
/// `tables` counts rows: for an append-only table the observations it has committed, for a derived
/// table the rows it currently materializes. `appended` counts observations alone, so it is zero for
/// every derived table, and `observations` sums it — evidence the store holds, never state it derived.
/// None of the three is the sequence pointer, which a batch that reserved and failed to commit leaves
/// ahead of the rows the store holds.
#[derive(Debug, Clone, Serialize)]
pub struct StoreStats {
    /// Rows each table holds, in [`Table::ALL`] order.
    pub tables: Vec<(String, u64)>,
    /// Observations each table has appended; zero for a table that derives its rows instead.
    pub appended: Vec<(String, u64)>,
    /// Observations the append-only tables hold: the sum of `appended`.
    pub observations: u64,
    /// LSM-tree level sizes as fjall reports them: SST files only, no write-ahead journal.
    pub bytes_on_disk: u64,
    /// Recursive size of the store root, journal, HTTP cache and outputs included.
    pub store_bytes: u64,
}

pub struct Store {
    root: PathBuf,
    db: Database,
    entities: Keyspace,
    journal: Keyspace,
    meta: Keyspace,
    /// Next observation sequence per table; seeded from each table's durable mark at open.
    sequences: sequences::Counters,
    /// Orders the batches that advance a table's mark, so the mark no batch can be overtaken by one
    /// that reserved later. See [`Store::lock_appends`].
    appends: Mutex<()>,
}

impl Store {
    pub fn open(root: impl AsRef<Path>) -> StoreResult<Self> {
        let root = root.as_ref().to_path_buf();
        for sub in ["http", "out"] {
            let dir = root.join(sub);
            std::fs::create_dir_all(&dir).map_err(|source| StoreError::Io {
                path: dir.clone(),
                source,
            })?;
        }
        let db = Database::builder(root.join(DB_DIR))
            .cache_size(CACHE_BYTES)
            .open()
            .map_err(|source| StoreError::Open { source })?;
        let entities = db
            .keyspace(ENTITIES, KeyspaceCreateOptions::default)
            .map_err(|source| StoreError::Open { source })?;
        let journal = db
            .keyspace(JOURNAL, KeyspaceCreateOptions::default)
            .map_err(|source| StoreError::Open { source })?;
        let meta = db
            .keyspace(META, KeyspaceCreateOptions::default)
            .map_err(|source| StoreError::Open { source })?;

        let sequences = sequences::Counters::seeded(&db, &entities, &meta)?;

        // Reclaim what a dead writer left behind. The lock above is exclusive, so any temporary
        // still on disk belongs to a process that is no longer running.
        read::sweep_stale_temporaries(&root)?;

        let store = Self {
            root,
            db,
            entities,
            journal,
            meta,
            sequences,
            appends: Mutex::new(()),
        };
        // Opening a store is a read, and the one-time import of a pre-Fjall corpus is a write: it is
        // [`Store::import_legacy`], and the paths that have decided to migrate call it — the offline
        // census run, the `import-legacy` verb, and the service bootstrap that owns the store for the
        // live route. Importing here made every open a writer, so an operator running a verb that
        // only measures a legacy root — integrity, backup, the restore drill — moved the corpus as a
        // side effect of looking at it.
        //
        // The row-count ledger is still seeded here, because it is a count of the rows this store
        // already holds rather than a migration of rows: a store written before the ledger existed
        // has no counts, and a count is only knowable by walking the table. It writes `rows:<table>`
        // for a table that has none, once, and nothing else.
        store.seed_row_marks()?;
        Ok(store)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn http_cache_dir(&self) -> PathBuf {
        self.root.join("http")
    }

    pub fn out_dir(&self) -> PathBuf {
        self.root.join("out")
    }

    /// Where a pre-Fjall store kept this table's append log. Reads no longer come from here; the
    /// path survives as the one-time import source and as the materialized export location.
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
}
#[cfg(test)]
#[path = "backup_tests.rs"]
mod backup_tests;
#[cfg(all(feature = "loom", test))]
mod loom_tests;
#[cfg(test)]
mod legacy_tests;
#[cfg(test)]
mod marks_tests;
#[cfg(test)]
mod tests;
#[cfg(kani)]
include!("../../kani/store_wiring.rs");
