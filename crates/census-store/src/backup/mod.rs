mod copy;
mod errors;
pub(super) mod files;
mod generation;
mod integrity;
mod manifest;
mod restore;
mod tree;

use std::collections::BTreeMap;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct BackupReport {
    pub to: String,
    pub files: u64,
    pub bytes: u64,
    pub tables: BTreeMap<String, u64>,
    pub elapsed_ms: u64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RestoreReport {
    pub from: String,
    pub to: String,
    pub files: u64,
    pub bytes: u64,
    pub tables: BTreeMap<String, u64>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IntegrityReport {
    pub ok: bool,
    pub tables: Vec<IntegrityTable>,
    pub unreadable_journals: Vec<String>,
    pub unreadable_entity_logs: Vec<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct IntegrityTable {
    pub table: String,
    pub expected: u64,
    pub actual: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub details: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct ManifestEntry {
    pub path: String,
    pub length: u64,
    pub sha256: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(super) struct Manifest {
    pub version: u32,
    pub written_at: String,
    pub files: Vec<ManifestEntry>,
    pub tables: BTreeMap<String, u64>,
}

pub(super) const MANIFEST_PATH: &str = "backup.json";

pub(super) const MANIFEST_VERSION: u32 = 2;

pub(super) const STAGING_PREFIX: &str = "backup.tmp.";

pub(super) const RESTORE_PREFIX: &str = "restore.tmp.";
