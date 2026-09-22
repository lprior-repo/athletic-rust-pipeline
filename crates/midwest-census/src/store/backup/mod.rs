//! Store backup, restore, and integrity checking.
//!
//! [`Store::backup`] copies the store's durable material into a destination directory and writes a
//! `backup.json` manifest listing every file with its sha256 digest, byte length, and per-table row
//! counts drawn from the same sequence counters that [`Store::stats`] uses.
//!
//! [`Store::restore`] validates the manifest against the backed-up files and materialises them into
//! a fresh directory, then re-opens through the normal [`Store::open`] path to report recovered
//! counts.
//!
//! [`Store::integrity`] checks every table's row count against its sequence counter, the journal
//! and entity-log pairings the store expects, and readability of each file.

mod copy;
mod helpers;
mod integrity;
mod restore;

use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Report types
// ---------------------------------------------------------------------------

/// What one [`Store::backup`] call produced.

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BackupReport {
    /// The backup was written here.
    pub to: String,
    /// Total files copied.
    pub files: u64,
    /// Total bytes copied.
    pub bytes: u64,
    /// Per-table row counts, the same sequence counters [`Store::stats`] reads.
    pub tables: BTreeMap<String, u64>,
    /// How long the backup took.
    pub elapsed_ms: u64,
}

/// What one [`Store::restore`] call produced.

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
    /// Per-table counts recovered by opening the restored store.
    pub tables: BTreeMap<String, u64>,
}

/// What one [`Store::integrity`] call found.

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IntegrityReport {
    /// True only when the mismatch list is empty.
    pub ok: bool,
    /// Per-table expected (sequence counter) vs actual (row count).
    pub tables: Vec<IntegrityTable>,
    /// Journal files that are unreadable.
    pub unreadable_journals: Vec<String>,
    /// Entity log files that are unreadable.
    pub unreadable_entity_logs: Vec<String>,
}

/// One table's expected vs actual count.

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IntegrityTable {
    /// Table name, e.g. `"schools"`.
    pub table: String,
    /// Expected from the sequence counter.
    pub expected: u64,
    /// Actual from the Fjall keyspace.
    pub actual: u64,
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

/// The `backup.json` manifest.

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(super) struct Manifest {
    /// Manifest version (future-proofing).
    pub version: u32,
    /// Manifest written at this time.
    pub written_at: String,
    /// All files in the backup.
    pub files: Vec<ManifestEntry>,
    /// Per-table row counts at backup time.
    pub tables: BTreeMap<String, u64>,
}

const MANIFEST_PATH: &str = "backup.json";
const MANIFEST_VERSION: u32 = 1;
