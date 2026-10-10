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
    #[serde(default)]
    pub links: u64,
    pub tables: BTreeMap<String, u64>,
    pub elapsed_ms: u64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct RestoreReport {
    pub from: String,
    pub to: String,
    pub files: u64,
    pub bytes: u64,
    #[serde(default)]
    pub links: u64,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
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

use std::path::Path;

pub(super) fn http_cache_integrity(root: &Path) -> crate::StoreResult<Vec<String>> {
    use std::fs;
    use sha2::{Sha256, Digest};

    let http_dir = root.join("http");
    if !http_dir.exists() {
        return Ok(Vec::new());
    }

    let mut issues = Vec::new();

    // Check bodies directory
    let bodies_dir = http_dir.join("bodies");
    if bodies_dir.exists() {
        for entry in fs::read_dir(&bodies_dir).map_err(|source| crate::StoreError::Io {
            path: bodies_dir.clone(),
            source,
        })?.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let file_name = path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
            if !file_name.ends_with(".body") {
                issues.push(format!("bodies/: unexpected file {file_name}"));
                continue;
            }
            let digest = file_name.strip_suffix(".body").unwrap_or("");
            if digest.len() != 64 {
                issues.push(format!("bodies/: invalid digest length in {file_name}"));
                continue;
            }
            let Ok(bytes) = fs::read(&path) else {
                issues.push(format!("bodies/: unreadable {file_name}"));
                continue;
            };
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let mut actual = String::new();
            for byte in hasher.finalize() {
                actual.push_str(&format!("{byte:02x}"));
            }
            if actual != digest {
                issues.push(format!(
                    "bodies/: digest mismatch in {} (expected {}... got {}...)",
                    file_name, &digest[..8], &actual[..8]
                ));
            }
        }
    }

    // Check meta files
    let meta_dir = http_dir.join("meta");
    if meta_dir.exists() {
        for entry in fs::read_dir(&meta_dir).map_err(|source| crate::StoreError::Io {
            path: meta_dir.clone(),
            source,
        })?.filter_map(|e| e.ok()) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let file_name = path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default();
            if !file_name.ends_with(".meta") {
                issues.push(format!("meta/: unexpected file {file_name}"));
                continue;
            }
            let Ok(content) = fs::read_to_string(&path) else {
                issues.push(format!("meta/: unreadable {file_name}"));
                continue;
            };
            let Ok(meta) = serde_json::from_str::<serde_json::Value>(&content) else {
                issues.push(format!("meta/: invalid JSON in {file_name}"));
                continue;
            };
            if let Some(body_digest) = meta.get("body_digest").and_then(|v| v.as_str()) {
                if body_digest.len() != 64 {
                    issues.push(format!("meta/: invalid digest in {file_name}"));
                    continue;
                }
                let body_path = bodies_dir.join(format!("{body_digest}.body"));
                if !body_path.exists() {
                    issues.push(format!("meta/: missing body for {file_name}"));
                }
            }
        }
    }

    Ok(issues)
}
pub(super) const RESTORE_PREFIX: &str = "restore.tmp.";
