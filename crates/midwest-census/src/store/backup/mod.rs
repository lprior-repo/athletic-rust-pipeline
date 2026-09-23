//! Store backup, restore, and integrity checking.
//!
//! [`Store::backup`] makes a **cold** copy of a closed store and writes `backup.json` beside it: every
//! file of the generation with its byte length and SHA-256, and the row count every table held when the
//! generation was finished.
//!
//! # Why the store has to be closed
//!
//! fjall 3.1.10 has no primitive that makes a file-level copy of a *live* database consistent, and this
//! module does not pretend otherwise:
//!
//! * `Database::snapshot()` (`fjall-3.1.10/src/db.rs:150`) returns an in-memory read view over the
//!   keyspaces (`src/snapshot.rs:17-31`). It pins the LSM version so reads see one point in time; it
//!   does not stop compaction from rewriting SSTs, and it does not stop the write-ahead journal from
//!   rotating or from being mid-frame when a copier reaches it.
//! * The crate has no `backup`, `checkpoint`, `export` or read-only-open entry point at all
//!   (`grep -rni 'backup' ~/.cargo/registry/src/*/fjall-3.1.10/src/` matches nothing), and no
//!   `Store::flush` equivalent of a freeze: `Database::persist(PersistMode::SyncAll)`
//!   (`src/db.rs:350`) fsyncs what is written, it does not stop writers.
//!
//! So [`Store::backup`] is what `tools/ops-backup-drill.sh` has always done by hand - stop the writer,
//! copy the stopped store - with the stop *enforced* instead of assumed. It takes `fjall/lock` with
//! `std::fs::File::try_lock`, the same advisory lock fjall itself takes when it opens the database
//! (`fjall-3.1.10/src/locked_file.rs:49-80`, `LOCK_FILE = "lock"` at `src/file.rs:11`, taken at
//! `src/db.rs:569`), and a lock that is already held **refuses the backup** with the message that says
//! so. Holding it for the length of the copy is also what stops a writer from opening the store halfway
//! through. A live recursive copy of an open LSM tree is not a snapshot, and is never published as one.
//!
//! # Generations
//!
//! A backup is built in a staging directory beside its destination (`backup.tmp.<token>/`) and renamed
//! onto it in one step only once every file, the manifest, and the digests are done. A failed run
//! therefore leaves an earlier backup exactly as it was, and re-running into an existing backup
//! directory replaces the whole generation rather than rewriting its files one at a time. [`Store::restore`]
//! is staged the same way (`restore.tmp.<token>/`), so a restore that fails at any point - including a
//! restored copy that does not open - leaves the destination as it found it and a retry unblocked.
//!
//! # What restore checks
//!
//! Restore streams every file once, checking its SHA-256 and length as the bytes pass (constant memory,
//! no second pass over the backup), opens the materialised tree as a store, and then reconciles the row
//! counts the restored keyspace actually holds against the manifest's. A restore that loses rows (an
//! engine that drops a segment on recovery) or invents them is refused rather than reported as a
//! success. The counts are the keyspace's own keys, counted per table prefix - not the store's sequence
//! pointers, which are not row counts once a commit fails or a derived table is written.
//!
//! [`Store::integrity`] checks the store in place: every table's ledger count against the rows its
//! keyspace holds, the invariant each table's write mode states — an append-only table's mark never
//! falling behind its keys, a derived table holding one row per id under sequence zero — and
//! readability of the journal and entity logs.

mod copy;
mod errors;
/// Byte-level copy and digest plumbing, visible in `store` because the backup tests drive it.
pub(super) mod files;
mod generation;
mod integrity;
mod manifest;
mod restore;
mod tree;

use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Report types
// ---------------------------------------------------------------------------

/// What one [`Store::backup`](crate::store::Store::backup) call produced.

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BackupReport {
    /// The backup was written here.
    pub to: String,
    /// Total files copied.
    pub files: u64,
    /// Total bytes copied.
    pub bytes: u64,
    /// Per-table row counts of the published generation, counted from the keys each table holds.
    pub tables: BTreeMap<String, u64>,
    /// How long the backup took.
    pub elapsed_ms: u64,
}

/// What one [`Store::restore`](crate::store::Store::restore) call produced.

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RestoreReport {
    /// The backup source directory.
    pub from: String,
    /// The restored store root.
    pub to: String,
    /// Files materialised.
    pub files: u64,
    /// Total bytes written.
    pub bytes: u64,
    /// Per-table row counts of the restored store, counted from the keys it holds and reconciled
    /// against the manifest's.
    pub tables: BTreeMap<String, u64>,
}

/// What one [`Store::integrity`](crate::store::Store::integrity) call found.

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IntegrityReport {
    /// True only when no table contradicts either its count or its mode's invariant, and every file
    /// the store's journal and entity directories name is readable.
    pub ok: bool,
    /// Per-table expected (the count the table's writer keeps) vs actual (rows in the keyspace).
    pub tables: Vec<IntegrityTable>,
    /// Journal files that are unreadable.
    pub unreadable_journals: Vec<String>,
    /// Entity log files that are unreadable.
    pub unreadable_entity_logs: Vec<String>,
}

/// One table's expected vs actual count, and any mode-specific fact a count cannot express.

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IntegrityTable {
    /// Table name, e.g. `"schools"`.
    pub table: String,
    /// Expected: the count the table's own batches keep.
    pub expected: u64,
    /// Actual: rows counted under the table's key prefix.
    pub actual: u64,
    /// Mode-specific facts a count cannot express: a sequence mark behind the keys it accounts for, a
    /// derived table holding a row keyed under a foreign sequence, or an id owning more than one row.
    /// Empty when the table agrees with the invariant its own write mode states.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<String>,
}

// ---------------------------------------------------------------------------
// Manifest
// ---------------------------------------------------------------------------

/// A single file entry in `backup.json`.

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct ManifestEntry {
    /// Relative path within the backup directory.
    pub path: String,
    /// Byte length of the file.
    pub length: u64,
    /// SHA-256 hex digest.
    pub sha256: String,
}

/// The `backup.json` manifest: every file of one published generation, and what it holds.

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(super) struct Manifest {
    /// Manifest version; a manifest whose version is not [`MANIFEST_VERSION`] is refused, not guessed at.
    pub version: u32,
    /// Manifest written at this time.
    pub written_at: String,
    /// All files in the backup.
    pub files: Vec<ManifestEntry>,
    /// Per-table row counts at backup time, counted from the keyspaces of the published generation.
    pub tables: BTreeMap<String, u64>,
}

pub(super) const MANIFEST_PATH: &str = "backup.json";

/// The manifest this build writes, and the only one it reads.
///
/// Version 1 recorded each table's *sequence pointer*. A pointer is not a row count - a commit that
/// fails leaves it ahead of the rows committed, and a derived table spends no sequence at all - so
/// version 2 records the rows the keyspace holds, which is what restore reconciles against. A version 1
/// manifest is refused rather than read with today's rules: re-take the backup with this build.
pub(super) const MANIFEST_VERSION: u32 = 2;

/// Staging directory of a generation being built, beside the destination it will be renamed onto.
pub(super) const STAGING_PREFIX: &str = "backup.tmp.";

/// Staging directory a restore is materialised in before it is renamed onto the destination.
pub(super) const RESTORE_PREFIX: &str = "restore.tmp.";
