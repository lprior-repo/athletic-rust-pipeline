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
//! ```
//!
//! Observations are append-only: appending the same entity twice writes two rows, and
//! [`Store::consolidate`] merges them through [`Entity::merge`], which is exactly the guarantee the
//! JSONL journals used to provide. The sequence component is **big-endian** so byte order is
//! numerical order, and it is seeded from the last key present at open time, so reopening a database
//! never reuses a sequence number and never overwrites an observation.
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
//! their resume ledger in `<store>/journal/*.jsonl`. [`Store::open`] imports both exactly once
//! (recorded under `meta`), skipping the import when the marker is present, so a partially imported
//! database finishes importing on the next open without duplicating observations.

use fjall::{Database, Keyspace, KeyspaceCreateOptions, PersistMode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

/// Store failures: fjall, row encoding, journal bounds, counters and sidecar I/O.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    /// The database or a keyspace could not be opened.
    #[error("store open failed: {source}")]
    Open {
        #[source]
        source: fjall::Error,
    },
    /// The write-ahead journal could not be flushed.
    #[error("store flush failed: {source}")]
    Flush {
        #[source]
        source: fjall::Error,
    },
    /// A read or range scan failed.
    #[error("store read failed: {source}")]
    Read {
        #[source]
        source: fjall::Error,
    },
    /// An append or batch commit failed.
    #[error("store write failed: {source}")]
    Write {
        #[source]
        source: fjall::Error,
    },
    /// A stored row is not valid JSON.
    #[error("row {key} is not valid json: {source}")]
    Decode {
        key: String,
        #[source]
        source: serde_json::Error,
    },
    /// A row did not encode to JSON, or unkeyed bytes did not decode: `detail` names what failed.
    #[error("{detail}: {source}")]
    Json {
        detail: String,
        #[source]
        source: serde_json::Error,
    },
    /// A key, counter or row id violated an invariant the store's writer maintains.
    #[error("{detail}")]
    Invariant { detail: String },
    /// One scan would exceed the configured row ceiling.
    #[error("table {table} would exceed {max} rows in one scan")]
    TooManyRows { table: String, max: usize },
    /// The resume journal exceeded its byte ceiling.
    #[error("resume journal exceeds {max} bytes")]
    JournalTooLarge { max: usize },
    /// The sequence counter at the end of its range.
    #[error("sequence counter overflow")]
    CounterOverflow,
    /// A sidecar or artifact file operation failed.
    #[error("i/o failed for {path}: {source}")]
    Io {
        path: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The one-time legacy journal import failed.
    #[error("legacy import failed: {detail}")]
    Legacy { detail: String },
}

/// Result alias for store code.
pub type StoreResult<T> = std::result::Result<T, StoreError>;

mod entities;
mod keys;
mod legacy;
pub mod read;
mod sequences;
mod write;

/// Hard ceiling on the observations one table may hold. A table larger than this aborts the scan
/// with a typed error instead of exhausting memory: the bound is what keeps Rule 2 (bounded control
/// flow) honest for a store whose input size is not known in advance.
pub const MAX_ROWS_PER_TABLE: u64 = 20_000_000;

/// Longest entity id the store accepts. Ids ride verbatim inside observation keys, and Fjall
/// asserts keys stay under 64 KiB; this ceiling keeps that assertion unreachable for callers.
pub const MAX_ID_BYTES: usize = 512;

/// Unified cache for the LSM tree. Bounded on purpose: the default is sized to the machine, and this
/// process is expected to share the machine with a browser and a text editor.
const CACHE_BYTES: u64 = 256 * 1024 * 1024;

const DB_DIR: &str = "fjall";
const ENTITIES: &str = "entities";
const JOURNAL: &str = "journal";
const META: &str = "meta";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Table {
    Schools,
    Teams,
    Coaches,
    Athletes,
    Meets,
    Events,
    Performances,
}

impl Table {
    pub fn file(self) -> &'static str {
        match self {
            Table::Schools => "schools",
            Table::Teams => "teams",
            Table::Coaches => "coaches",
            Table::Athletes => "athletes",
            Table::Meets => "meets",
            Table::Events => "events",
            Table::Performances => "performances",
        }
    }

    pub const ALL: [Table; 7] = [
        Table::Schools,
        Table::Teams,
        Table::Coaches,
        Table::Athletes,
        Table::Meets,
        Table::Events,
        Table::Performances,
    ];

    /// Parse a wire name (`"schools"`) back into a table. Unknown names are rejected so a typo in an
    /// ingest request cannot silently create a table nobody scans.
    pub fn from_wire(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|table| table.file() == name)
    }
}

/// An entity that knows its own canonical id and how to absorb a duplicate observation.
pub trait Entity: Serialize + DeserializeOwned + Clone {
    fn entity_id(&self) -> &str;
    fn merge(&mut self, other: Self);

    /// Apply the collection contract to a merged entity. Every read of the store goes through
    /// [`Store::scan`], so a rule that lives here holds for the report, the workbook, the snapshot
    /// and the Restate handlers at once.
    fn publish(&mut self) {}

    /// How many of this entity's rows carry something the contract withheld. [`Store::consolidate`]
    /// sums this in the same pass that writes the snapshot, so reporting the count never re-scans
    /// the table.
    fn withheld_mailboxes(&self) -> usize {
        0
    }
}

/// What one [`Store::consolidate`] call produced: the rows written, and how many of them the
/// collection contract withheld a consumer mailbox from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Consolidated {
    pub rows: usize,
    pub withheld: usize,
}

/// Per-table row counts and the database's on-disk footprint. Counts are the LSM tree's own
/// estimates (`approximate_len`), which is what a status command needs without scanning millions of
/// rows.
#[derive(Debug, Clone, Serialize)]
pub struct StoreStats {
    pub tables: Vec<(String, u64)>,
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
    /// Next observation sequence per table; seeded from the last key found at open.
    sequences: sequences::Counters,
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

        let sequences = sequences::Counters::seeded(&entities)?;

        let store = Self {
            root,
            db,
            entities,
            journal,
            meta,
            sequences,
        };
        store.import_legacy()?;
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
#[cfg(all(feature = "loom", test))]
mod loom_tests;
#[cfg(test)]
mod tests;
#[cfg(kani)]
include!("../../kani/store_wiring.rs");
