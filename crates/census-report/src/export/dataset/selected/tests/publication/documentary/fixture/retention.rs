use super::{files, Handoff, TestResult};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

pub(in super::super) fn retain_v3_archive(
    dataset: &crate::export::ExportDataset,
    archive_directory: &Path,
    frozen: &Path,
    digest: &str,
    lineage: &impl serde::Serialize,
    observations: &serde_json::Value,
) -> TestResult {
    let Some(destination) = std::env::var_os("CENSUS_DOCUMENTARY_V3_ARCHIVE_DIR") else {
        return Ok(());
    };
    super::super::historical_retained::require_v3(dataset, frozen)?;
    let destination = PathBuf::from(destination);
    require_destination(&destination)?;
    let report_commit = std::env::var("CENSUS_DOCUMENTARY_REPORT_PRODUCER_COMMIT")?;
    let report_tree = std::env::var("CENSUS_DOCUMENTARY_REPORT_PRODUCER_TREE")?;
    check!(files::hex(&report_commit, 40));
    check!(files::hex(&report_tree, 40));
    let (handoff, handoff_digest) = handoff()?;
    for source in [handoff.store_path.as_path(), archive_directory] {
        let source = std::fs::canonicalize(source)?;
        check!(!destination.starts_with(&source));
        check!(!source.starts_with(&destination));
    }
    files::verify(&handoff.store_path, &handoff.files)?;
    let inventory = files::inventory(archive_directory)?;
    check!(eq; frozen.parent(), Some(archive_directory));
    check!(eq; frozen.file_name().and_then(|name| name.to_str()), Some("genuine-documentary.json"));
    check!(inventory
        .iter()
        .any(|file| file.path == "genuine-documentary.json"));
    check!(inventory
        .iter()
        .any(|file| file.path.starts_with("genuine-documentary.json.proofs/")));
    let manifest = json!({
        "schema": "genuine-documentary-v3-retained-archive/v1",
        "report_producer_commit": report_commit,
        "report_producer_tree": report_tree,
        "store_handoff_sha256": handoff_digest,
        "store_producer_commit": handoff.producer_commit,
        "store_producer_tree": handoff.producer_tree,
        "store_identity": handoff.store_identity,
        "store_snapshot_sequence": handoff.snapshot_sequence,
        "store_tables_digest": handoff.tables_digest,
        "store_inventory": handoff.inventory,
        "store_files": rows(&handoff.files),
        "archive": "archive/genuine-documentary.json",
        "archive_digest": digest,
        "lineage": lineage,
        "observations": observations,
        "archive_files": rows(&inventory),
    });
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    check!(bytes.len() <= 32 * 1024 * 1024);
    std::fs::create_dir(&destination)?;
    files::copy(archive_directory, &destination.join("archive"), &inventory)?;
    files::verify(&handoff.store_path, &handoff.files)?;
    let path = destination.join("retention.json");
    let mut output = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    output.write_all(&bytes)?;
    output.sync_all()?;
    eprintln!(
        "GENUINE_DOCUMENTARY_V3_RETENTION {}",
        serde_json::to_string(&json!({
            "manifest": path, "sha256": format!("{:x}", Sha256::digest(&bytes)),
        }))?
    );
    Ok(())
}

fn require_destination(destination: &Path) -> TestResult {
    check!(destination.is_absolute());
    check!(destination
        .components()
        .all(|part| matches!(part, Component::RootDir | Component::Normal(_))));
    let parent = destination.parent().ok_or("retention destination parent")?;
    check!(eq; std::fs::canonicalize(parent)?, parent);
    check!(std::fs::symlink_metadata(parent)?.file_type().is_dir());
    check!(!destination.starts_with(std::fs::canonicalize(std::env::current_dir()?)?));
    check!(
        matches!(std::fs::symlink_metadata(destination), Err(error) if error.kind() == std::io::ErrorKind::NotFound)
    );
    Ok(())
}

fn handoff() -> TestResult<(Handoff, String)> {
    let path =
        std::env::var_os("CENSUS_DOCUMENTARY_HANDOFF_JSON").ok_or("genuine Store handoff")?;
    let metadata = std::fs::symlink_metadata(&path)?;
    check!(metadata.file_type().is_file());
    check!(metadata.len() <= 32 * 1024 * 1024);
    let file = std::fs::File::open(path)?;
    check!(eq; file.metadata()?.len(), metadata.len());
    let mut bytes = Vec::new();
    file.take(32 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
    check!(bytes.len() <= 32 * 1024 * 1024);
    check!(eq; u64::try_from(bytes.len())?, metadata.len());
    let handoff: Handoff = serde_json::from_slice(&bytes)?;
    check!(eq; handoff.schema, "genuine-documentary-report-handoff/v1");
    check!(eq; handoff.producer_commit, std::env::var("CENSUS_DOCUMENTARY_PRODUCER_COMMIT")?);
    check!(eq; handoff.producer_tree, std::env::var("CENSUS_DOCUMENTARY_PRODUCER_TREE")?);
    Ok((handoff, format!("{:x}", Sha256::digest(&bytes))))
}

fn rows(files: &[files::File]) -> Vec<serde_json::Value> {
    files
        .iter()
        .map(|file| json!({"path": file.path, "bytes": file.bytes, "sha256": file.sha256}))
        .collect()
}
