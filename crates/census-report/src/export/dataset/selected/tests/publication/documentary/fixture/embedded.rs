use super::{files, Handoff, TestResult};
use std::path::Path;

const MANIFEST: &str = include_str!("embedded_manifest.json");

const FJALL_0_JNL: &[u8] = include_bytes!("embedded_store/fjall/0.jnl");
const KEYSPACES_0_CURRENT: &[u8] = include_bytes!("embedded_store/fjall/keyspaces/0/current");
const KEYSPACES_0_TABLES_3: &[u8] = include_bytes!("embedded_store/fjall/keyspaces/0/tables/3");
const KEYSPACES_0_TABLES_4: &[u8] = include_bytes!("embedded_store/fjall/keyspaces/0/tables/4");
const KEYSPACES_0_V6: &[u8] = include_bytes!("embedded_store/fjall/keyspaces/0/v6");
const KEYSPACES_1_CURRENT: &[u8] = include_bytes!("embedded_store/fjall/keyspaces/1/current");
const KEYSPACES_1_V0: &[u8] = include_bytes!("embedded_store/fjall/keyspaces/1/v0");
const KEYSPACES_2_CURRENT: &[u8] = include_bytes!("embedded_store/fjall/keyspaces/2/current");
const KEYSPACES_2_V0: &[u8] = include_bytes!("embedded_store/fjall/keyspaces/2/v0");
const KEYSPACES_3_CURRENT: &[u8] = include_bytes!("embedded_store/fjall/keyspaces/3/current");
const KEYSPACES_3_V0: &[u8] = include_bytes!("embedded_store/fjall/keyspaces/3/v0");
const KEYSPACES_4_CURRENT: &[u8] = include_bytes!("embedded_store/fjall/keyspaces/4/current");
const KEYSPACES_4_V0: &[u8] = include_bytes!("embedded_store/fjall/keyspaces/4/v0");
const FJALL_LOCK: &[u8] = include_bytes!("embedded_store/fjall/lock");
const FJALL_VERSION: &[u8] = include_bytes!("embedded_store/fjall/version");

const STORE_BYTES: &[(&str, &[u8])] = &[
    ("fjall/0.jnl", FJALL_0_JNL),
    ("fjall/keyspaces/0/current", KEYSPACES_0_CURRENT),
    ("fjall/keyspaces/0/tables/3", KEYSPACES_0_TABLES_3),
    ("fjall/keyspaces/0/tables/4", KEYSPACES_0_TABLES_4),
    ("fjall/keyspaces/0/v6", KEYSPACES_0_V6),
    ("fjall/keyspaces/1/current", KEYSPACES_1_CURRENT),
    ("fjall/keyspaces/1/v0", KEYSPACES_1_V0),
    ("fjall/keyspaces/2/current", KEYSPACES_2_CURRENT),
    ("fjall/keyspaces/2/v0", KEYSPACES_2_V0),
    ("fjall/keyspaces/3/current", KEYSPACES_3_CURRENT),
    ("fjall/keyspaces/3/v0", KEYSPACES_3_V0),
    ("fjall/keyspaces/4/current", KEYSPACES_4_CURRENT),
    ("fjall/keyspaces/4/v0", KEYSPACES_4_V0),
    ("fjall/lock", FJALL_LOCK),
    ("fjall/version", FJALL_VERSION),
];

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: String,
    producer_commit: String,
    producer_tree: String,
    store_identity: String,
    snapshot_sequence: u64,
    tables_digest: String,
    inventory: serde_json::Value,
    files: Vec<files::File>,
    provenance: Provenance,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Provenance {
    produced_by: String,
    produced_at_commit: String,
    produced_at_tree: String,
    note: String,
}

pub(in super::super) fn materialize(original: &Path) -> TestResult<Handoff> {
    let manifest: Manifest = serde_json::from_str(MANIFEST)?;
    check!(eq; manifest.schema, "genuine-documentary-report-handoff/v1");
    check!(files::hex(&manifest.producer_commit, 40));
    check!(files::hex(&manifest.producer_tree, 40));
    check!(eq; manifest.producer_commit, manifest.provenance.produced_at_commit);
    check!(eq; manifest.producer_tree, manifest.provenance.produced_at_tree);
    check!(!manifest.provenance.produced_by.is_empty());
    check!(!manifest.provenance.note.is_empty());
    check!(eq; STORE_BYTES.len(), manifest.files.len());
    for ((path, bytes), file) in STORE_BYTES.iter().zip(&manifest.files) {
        check!(eq; *path, file.path);
        check!(eq; u64::try_from(bytes.len())?, file.bytes);
    }
    std::fs::create_dir_all(original)?;
    for (path, bytes) in STORE_BYTES {
        let target = original.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, bytes)?;
    }
    files::verify(original, &manifest.files)?;
    Ok(Handoff {
        schema: manifest.schema,
        producer_commit: manifest.producer_commit,
        producer_tree: manifest.producer_tree,
        store_path: original.to_path_buf(),
        store_identity: manifest.store_identity,
        snapshot_sequence: manifest.snapshot_sequence,
        tables_digest: manifest.tables_digest,
        inventory: manifest.inventory,
        files: manifest.files,
    })
}
