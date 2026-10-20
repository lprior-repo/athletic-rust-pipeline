use super::{files, TestResult};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const MANIFEST: &str = include_str!("embedded_v3_manifest.json");

const V3_EMBEDDED_V3_ARCHIVE_GENUINE_DOCUMENTARY_JSON: &[u8] =
    include_bytes!("embedded_v3_archive/genuine-documentary.json");
const V3_P0: &[u8] = include_bytes!("embedded_v3_archive/genuine-documentary.json.proofs/1dd80c34c5cc520a8c619e8f3387b215f0ff6d575eca764d31f650cf577dd105");
const V3_P1: &[u8] = include_bytes!("embedded_v3_archive/genuine-documentary.json.proofs/24c98ec93ca14f8373e3280db51a15499281338597f201528ef120eb969a4045");
const V3_P2: &[u8] = include_bytes!("embedded_v3_archive/genuine-documentary.json.proofs/56f4ad31eaea7db01c6031fc01f5b032eb8406157d0b88867d4b4df92086e186");
const V3_P3: &[u8] = include_bytes!("embedded_v3_archive/genuine-documentary.json.proofs/5eee176a301a55455b1c54a359624d1002aa770483a3bd308810aa85d58864c0");
const V3_P4: &[u8] = include_bytes!("embedded_v3_archive/genuine-documentary.json.proofs/6d59b3e7cf9f3b9c685dabc3707a5e600c2555e518b39044869af766f6cd47ac");
const V3_P5: &[u8] = include_bytes!("embedded_v3_archive/genuine-documentary.json.proofs/79c2d9fa7570857c03cdbacf7a2b37fd357633bb962ccf9b81677955493564c7");
const V3_P6: &[u8] = include_bytes!("embedded_v3_archive/genuine-documentary.json.proofs/7b752e9ccd819080442896a43b79fc1f4326627b46689ff847305d3830e4c6fb");
const V3_P7: &[u8] = include_bytes!("embedded_v3_archive/genuine-documentary.json.proofs/88d4a01f0b91231a522dfc2cea75e03a50ae4e7ce14cc304c2e3802dfff832c6");
const V3_P8: &[u8] = include_bytes!("embedded_v3_archive/genuine-documentary.json.proofs/8c4337beb338f6c990cdce3d32576330211038b353224e4b5cce0dbb84aaa439");
const V3_P9: &[u8] = include_bytes!("embedded_v3_archive/genuine-documentary.json.proofs/8e2740898236ed207b8195594ef35b338730ca3d574cd2c376eca10c34f98a60");
const V3_PA: &[u8] = include_bytes!("embedded_v3_archive/genuine-documentary.json.proofs/ad2d657139df1dca362a42a257bf2238f34df5f5258b72c4b8f2bec7951dd754");
const V3_PB: &[u8] = include_bytes!("embedded_v3_archive/genuine-documentary.json.proofs/bf3ade0c628ce113507efcdb5f54011caf561c5de2ebb9ffc4f8f0fbf0db0942");
const V3_PC: &[u8] = include_bytes!("embedded_v3_archive/genuine-documentary.json.proofs/dca394853e614db44e26cf47cf4f67d67e0968134abfdbf262eb9c288fc2de84");
const V3_PD: &[u8] = include_bytes!("embedded_v3_archive/genuine-documentary.json.proofs/ea0baa8f18106b23d4a40f82b252be857005bac3f4fffc7ebb3bfe138d0465af");

const ARCHIVE_BYTES: &[(&str, &[u8])] = &[
    (
        "genuine-documentary.json",
        V3_EMBEDDED_V3_ARCHIVE_GENUINE_DOCUMENTARY_JSON,
    ),
    (
        "genuine-documentary.json.proofs/1dd80c34c5cc520a8c619e8f3387b215f0ff6d575eca764d31f650cf577dd105",
        V3_P0,
    ),
    (
        "genuine-documentary.json.proofs/24c98ec93ca14f8373e3280db51a15499281338597f201528ef120eb969a4045",
        V3_P1,
    ),
    (
        "genuine-documentary.json.proofs/56f4ad31eaea7db01c6031fc01f5b032eb8406157d0b88867d4b4df92086e186",
        V3_P2,
    ),
    (
        "genuine-documentary.json.proofs/5eee176a301a55455b1c54a359624d1002aa770483a3bd308810aa85d58864c0",
        V3_P3,
    ),
    (
        "genuine-documentary.json.proofs/6d59b3e7cf9f3b9c685dabc3707a5e600c2555e518b39044869af766f6cd47ac",
        V3_P4,
    ),
    (
        "genuine-documentary.json.proofs/79c2d9fa7570857c03cdbacf7a2b37fd357633bb962ccf9b81677955493564c7",
        V3_P5,
    ),
    (
        "genuine-documentary.json.proofs/7b752e9ccd819080442896a43b79fc1f4326627b46689ff847305d3830e4c6fb",
        V3_P6,
    ),
    (
        "genuine-documentary.json.proofs/88d4a01f0b91231a522dfc2cea75e03a50ae4e7ce14cc304c2e3802dfff832c6",
        V3_P7,
    ),
    (
        "genuine-documentary.json.proofs/8c4337beb338f6c990cdce3d32576330211038b353224e4b5cce0dbb84aaa439",
        V3_P8,
    ),
    (
        "genuine-documentary.json.proofs/8e2740898236ed207b8195594ef35b338730ca3d574cd2c376eca10c34f98a60",
        V3_P9,
    ),
    (
        "genuine-documentary.json.proofs/ad2d657139df1dca362a42a257bf2238f34df5f5258b72c4b8f2bec7951dd754",
        V3_PA,
    ),
    (
        "genuine-documentary.json.proofs/bf3ade0c628ce113507efcdb5f54011caf561c5de2ebb9ffc4f8f0fbf0db0942",
        V3_PB,
    ),
    (
        "genuine-documentary.json.proofs/dca394853e614db44e26cf47cf4f67d67e0968134abfdbf262eb9c288fc2de84",
        V3_PC,
    ),
    (
        "genuine-documentary.json.proofs/ea0baa8f18106b23d4a40f82b252be857005bac3f4fffc7ebb3bfe138d0465af",
        V3_PD,
    ),
];

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: String,
    report_producer_commit: String,
    report_producer_tree: String,
    store_handoff_sha256: String,
    store_producer_commit: String,
    store_producer_tree: String,
    store_identity: String,
    store_snapshot_sequence: u64,
    store_tables_digest: String,
    store_inventory: serde_json::Value,
    store_files: Vec<files::File>,
    archive: String,
    archive_digest: String,
    lineage: serde_json::Value,
    observations: serde_json::Value,
    archive_files: Vec<files::File>,
}

pub(super) fn materialize(bundle_dir: &Path) -> TestResult<(PathBuf, String)> {
    let manifest: Manifest = serde_json::from_str(MANIFEST)?;
    check!(eq; manifest.schema, "genuine-documentary-v3-retained-archive/v1");
    check!(files::hex(&manifest.report_producer_commit, 40));
    check!(files::hex(&manifest.report_producer_tree, 40));
    check!(files::hex(&manifest.store_producer_commit, 40));
    check!(files::hex(&manifest.store_producer_tree, 40));
    check!(eq; manifest.archive_files.len(), ARCHIVE_BYTES.len());
    for ((path, bytes), file) in ARCHIVE_BYTES.iter().zip(&manifest.archive_files) {
        check!(eq; file.path.as_str(), *path);
        check!(eq; u64::try_from(bytes.len())?, file.bytes);
    }
    std::fs::create_dir_all(bundle_dir)?;
    std::fs::write(bundle_dir.join("retention.json"), MANIFEST)?;
    let archive = bundle_dir.join("archive");
    for (path, bytes) in ARCHIVE_BYTES {
        let target = archive.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, bytes)?;
    }
    files::verify(&archive, &manifest.archive_files)?;
    let manifest_path = bundle_dir.join("retention.json");
    let sha256 = format!("{:x}", Sha256::digest(MANIFEST.as_bytes()));
    Ok((manifest_path, sha256))
}

pub(in super::super) fn provision_retained_bundle() -> TestResult<()> {
    if std::env::var_os("CENSUS_DOCUMENTARY_V3_RETENTION_JSON").is_some() {
        return Ok(());
    }
    let holder = tempfile::tempdir()?;
    let (manifest_path, sha256) = materialize(holder.path())?;
    let manifest: Manifest = serde_json::from_str(MANIFEST)?;
    std::env::set_var("CENSUS_DOCUMENTARY_V3_RETENTION_JSON", &manifest_path);
    std::env::set_var("CENSUS_DOCUMENTARY_V3_RETENTION_SHA256", &sha256);
    std::env::set_var(
        "CENSUS_DOCUMENTARY_V3_REPORT_PRODUCER_COMMIT",
        &manifest.report_producer_commit,
    );
    std::env::set_var(
        "CENSUS_DOCUMENTARY_V3_REPORT_PRODUCER_TREE",
        &manifest.report_producer_tree,
    );
    std::env::set_var(
        "CENSUS_DOCUMENTARY_PRODUCER_COMMIT",
        &manifest.store_producer_commit,
    );
    std::env::set_var(
        "CENSUS_DOCUMENTARY_PRODUCER_TREE",
        &manifest.store_producer_tree,
    );
    std::mem::forget(holder);
    Ok(())
}
